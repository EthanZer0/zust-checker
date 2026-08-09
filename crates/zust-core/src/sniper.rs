//! EnrollSniper — 自动抢课引擎
//!
//! 对应 Python newjwxt/sniper.py
//!
//! 设计模式：后端自驱动轮询。前端只调用一次 `sniper_start(targets, interval_ms)`，
//! 后端 spawn 一个 tokio 任务循环：每轮对每个 target 调用 enroll_course，通过
//! `sniper-tick` 事件把本轮结果推给前端，然后 sleep(interval_ms) 进入下一轮。
//! 抢到任一目标（success）或收到 stop 时退出循环并发 `sniper-success`/`sniper-stopped`。
//! 引擎自身持有 targets、计数器与 per-target 统计，运行节奏由 `sniper_start` 的
//! loop 掌控；`tick()` 是单轮核心逻辑，可独立测试。
//!
//! 单轮内对多个 target **并发**发起 enroll_course（通过 Arc<JWClient> + tokio::spawn），
//! 轮间由调用方施加 interval + jitter，避免同一 session 的 N 次请求挤进同一毫秒窗口。

use crate::client::JWClient;
use crate::types::{SniperStats, SniperTarget, SniperTargetStatus, SniperTickResult};
use crate::error::Result;
use serde_json::Value as JsonValue;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// 抢课引擎
pub struct EnrollSniper {
    client: Arc<Mutex<JWClient>>,
    targets: Vec<SniperTarget>,
    running: Arc<AtomicBool>,
    total_attempts: Arc<AtomicU64>,
    start_time: Instant,
    stats: Arc<Mutex<Vec<SniperStats>>>,
}

impl EnrollSniper {
    /// 创建抢课引擎
    pub fn new(client: JWClient) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            targets: Vec::new(),
            running: Arc::new(AtomicBool::new(false)),
            total_attempts: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            stats: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 设置抢课目标
    pub fn set_targets(&mut self, targets: Vec<SniperTarget>) {
        self.stats = Arc::new(Mutex::new(
            targets
                .iter()
                .map(|t| SniperStats {
                    kch_id: t.kch_id.clone(),
                    kcmc: t.kcmc.clone(),
                    attempts: 0,
                    last_status: "pending".to_string(),
                    last_detail: String::new(),
                })
                .collect(),
        ));
        self.targets = targets;
    }

    /// 是否正在运行
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> Vec<SniperStats> {
        self.stats.lock().unwrap().clone()
    }

    /// 设置运行状态（用于外部 stop）
    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
        if running {
            // 重新开始计时
            // start_time 在首次 snipe_start 时设置
        }
    }

    /// 单轮 tick：**并发**对每个 target 调用一次 enroll_course，返回结果。
    /// 在轮内并发、轮间间隔的模型下，单轮的总延迟接近最慢的 target（而非串行之和）加上 stagger。
    ///
    /// `stagger_ms` — 每个 target 发出请求前多等 `index × stagger_ms`，把同一轮内的
    /// N 次 POST 错开到不同毫秒，避免触发方正"选课频率过高"限流。0 = 无错开。
    pub async fn tick(&self, stagger_ms: u64) -> Result<SniperTickResult> {
        let client = Arc::clone(&self.client);
        let total_attempts = Arc::clone(&self.total_attempts);
        let stats = Arc::clone(&self.stats);

        // Clone targets into owned Vec so spawned futures don't borrow &self.
        let targets: Vec<SniperTarget> = self.targets.clone();

        let futures: Vec<_> = targets
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let stagger = stagger_ms as u64 * index as u64;
                let client = Arc::clone(&client);
                let total_attempts = Arc::clone(&total_attempts);
                let stats = Arc::clone(&stats);
                let target = target.clone(); // full ownership for the spawn
                let kch_id = target.kch_id.clone();
                let kcmc = target.kcmc.clone();

                tokio::spawn(async move {
                    // ═══ Stagger：按 index 延迟，把并发请求错开到不同毫秒 ═══
                    if stagger > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(stagger)).await;
                    }

                    // Take everything we need from the mutex-protected client
                    // and from target before the .await boundary.
                    // MutexGuard is NOT Send, so we must drop it before awaiting.
                    let jw_client = {
                        let c = client.lock().unwrap();
                        c.clone()
                    }; // MutexGuard dropped — safe to await now
                    let t = target.clone();

                    let resp = jw_client
                        .enroll_course(
                            &t.jxb_ids, &t.kch_id, &t.kcmc,
                            &t.xkkz_id, &t.kklxdm,
                            &t.xkxnm, &t.xkxqm,
                            &t.njdm_id, &t.zyh_id,
                        )
                        .await
                        .unwrap_or_else(|e| {
                            let mut m = serde_json::Map::new();
                            m.insert("_raw".into(), JsonValue::String(format!("Error: {e}")));
                            JsonValue::Object(m)
                        });

                    let (status, detail) = parse_enroll_result(&resp, &target);

                    total_attempts.fetch_add(1, Ordering::SeqCst);

                    {
                        let mut s = stats.lock().unwrap();
                        if let Some(st) = s.iter_mut().find(|st| st.kch_id == kch_id) {
                            st.attempts += 1;
                            st.last_status = status.clone();
                            st.last_detail = detail.clone();
                        }
                    }

                    SniperTargetStatus { kch_id, kcmc, status, detail }
                })
            })
            .collect();

        // collect all handles
        let mut results: Vec<SniperTargetStatus> = Vec::with_capacity(futures.len());
        for handle in futures {
            match handle.await {
                Ok(stat) => results.push(stat),
                Err(e) => {
                    // tokio::spawn join error — shouldn't happen, but surface it
                    log::error!("sniper tick join error: {e}");
                }
            }
        }

        let elapsed = self.start_time.elapsed().as_secs_f64();

        Ok(SniperTickResult {
            session_valid: true, // overridden if any response indicates session expiry
            targets: results,
            total_attempts: self.total_attempts.load(Ordering::SeqCst),
            elapsed_secs: elapsed,
        })
    }

    /// 获取总尝试次数
    pub fn total_attempts(&self) -> u64 {
        self.total_attempts.load(Ordering::SeqCst)
    }

    /// 获取运行时长
    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// 重置计数器（新的一轮抢课）
    pub fn reset(&mut self) {
        self.total_attempts.store(0, Ordering::SeqCst);
        self.start_time = Instant::now();
    }

    /// 获取 client 的 clone（Arc<Mutex<JWClient>> → clone of Arc）
    pub fn client_ref(&self) -> Arc<Mutex<JWClient>> {
        Arc::clone(&self.client)
    }

    /// 替换内部的 JWClient（用于 session 过期后重新登录刷新 client）
    pub fn update_client(&self, new_client: JWClient) {
        let mut c = self.client.lock().unwrap();
        *c = new_client;
    }

    /// 获取 targets 引用
    pub fn targets(&self) -> &[SniperTarget] {
        &self.targets
    }
}

/// 解析选课 API 响应
///
/// 对应 Python EnrollSniper._parse_result()
///
/// 返回 (status, detail)，status 为: success / conflict / rate_limited / unavailable / session_expired / unknown
fn parse_enroll_result(r: &JsonValue, _target: &SniperTarget) -> (String, String) {
    // ═══ Session 失效 / 非 JSON 广义检测（选课 API 正常应返回 JSON， ═══
    // ═══ 若 _raw 里是 HTML 页面或已知失效特征，直接判为 session expired  ═══
    if let Some(raw) = r.get("_raw").and_then(|r| r.as_str()) {
        let raw_lower = raw.to_lowercase();

        // ── 广义：非 JSON（HTML 页面）兜底 ──
        // 选课接口正常返回 {"flag":"1"} 这类 JSON。一旦出现 HTML 结构特征，
        // 说明被重定向到了登录/错误页面 → session 已失效。
        let is_html = raw_lower.contains("<!doctype")
            || raw_lower.contains("<html")
            || raw_lower.contains("<head")
            || raw_lower.contains("<body")
            || raw_lower.contains("<meta")
            || raw_lower.contains("<script")
            || raw_lower.contains("<link")
            || raw_lower.contains("<form")
            || raw_lower.contains("<input")
            || raw_lower.contains("login");
        if is_html {
            let snippet = raw.chars().take(120).collect::<String>();
            return ("session_expired".into(), format!("Session 失效（返回 HTML 页面）: {snippet}"));
        }

        // ── 已知中文关键词 ──
        if raw_lower.contains("尚未分配身份")
            || raw_lower.contains("请重新登录")
            || raw_lower.contains("会话已过期")
            || raw_lower.contains("session expired")
            || raw_lower.contains("core/login/auth")
        {
            return (
                "session_expired".into(),
                "Session 已失效，请重新登录".into(),
            );
        }
    }

    // ── 频率限制检测（"选课频率过高" / "请稍后重试" 是瞬态，下轮再试即可）─
    {
        let check = |text: &str| -> bool {
            text.contains("频率过高") || text.contains("请稍后重试") || text.contains("rate limit")
        };
        // 情况 1: flag==0 且 msg 含限流关键词
        if let Some(flag) = r.get("flag").and_then(|f| f.as_str()) {
            if flag == "0" {
                if let Some(msg) = r.get("msg").and_then(|m| m.as_str()) {
                    if check(msg) {
                        return ("rate_limited".into(), msg.to_string());
                    }
                }
            }
        }
        // 情况 2: message 字段含限流关键词
        if let Some(msg) = r.get("message").and_then(|m| m.as_str()) {
            if check(msg) {
                return ("rate_limited".into(), msg.to_string());
            }
        }
        // 情况 3: _raw 回退
        if let Some(raw) = r.get("_raw").and_then(|r| r.as_str()) {
            if check(raw) {
                return ("rate_limited".into(), raw.chars().take(200).collect());
            }
        }
    }

    // 情况 1: dict 带 "flag" 字段
    if let Some(flag) = r.get("flag").and_then(|f| f.as_str()) {
        match flag {
            "1" => return ("success".into(), "选课成功".into()),
            "-1" => {
                // 检查是否是容量不足
                if let Some(code) = r.get("code").and_then(|c| c.as_str()) {
                    if code == "1" {
                        return ("unavailable".into(), "容量已满".into());
                    }
                }
                return ("unavailable".into(), "不可选".into());
            }
            "0" => {
                // 可能是冲突或不可选
                let msg = r
                    .get("msg")
                    .and_then(|m| m.as_str())
                    .unwrap_or("未知原因");
                if msg.contains("冲突") || msg.contains("已选") {
                    return ("conflict".into(), msg.to_string());
                }
                return ("unavailable".into(), msg.to_string());
            }
            _ => {}
        }
    }

    // 情况 2: dict 带 "message" 字段
    if let Some(msg) = r.get("message").and_then(|m| m.as_str()) {
        if msg.contains("成功") {
            return ("success".into(), msg.to_string());
        }
        if msg.contains("冲突") || msg.contains("已选") {
            return ("conflict".into(), msg.to_string());
        }
    }

    // 情况 3: _raw 字符串回退
    if let Some(raw) = r.get("_raw").and_then(|r| r.as_str()) {
        if raw.contains("成功") || raw.contains(r#""flag":"1""#) {
            return ("success".into(), raw.chars().take(200).collect());
        }
        if raw.contains("冲突") || raw.contains("已选") {
            return ("conflict".into(), raw.chars().take(200).collect());
        }
        if raw.contains("容量") || raw.contains("已满") || raw.contains("不可选") {
            return (
                "unavailable".into(),
                raw.chars().take(200).collect(),
            );
        }
        return ("unknown".into(), raw.chars().take(200).collect());
    }

    // 情况 4: 纯字符串
    if let Some(s) = r.as_str() {
        let trimmed = s.trim();
        if trimmed == "1" || trimmed == "success" || trimmed.contains("成功") {
            return ("success".into(), trimmed.to_string());
        }
        return ("unknown".into(), trimmed.to_string());
    }

    // 回退
    let detail = serde_json::to_string(r)
        .unwrap_or_default();
    let detail_short: String = detail.chars().take(200).collect();
    ("unknown".into(), detail_short)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_flag_1_success() {
        let r = serde_json::json!({"flag": "1"});
        let (status, _detail) = parse_enroll_result(
            &r,
            &SniperTarget::default(),
        );
        assert_eq!(status, "success");
    }

    #[test]
    fn test_parse_flag_0_conflict() {
        let r = serde_json::json!({"flag": "0", "msg": "已选冲突"});
        let (status, _detail) = parse_enroll_result(
            &r,
            &SniperTarget::default(),
        );
        assert_eq!(status, "conflict");
    }

    #[test]
    fn test_parse_raw_success() {
        let r = serde_json::json!({"_raw": "\"flag\":\"1\" 选课成功"});
        let (status, _detail) = parse_enroll_result(
            &r,
            &SniperTarget::default(),
        );
        assert_eq!(status, "success");
    }

    #[test]
    fn test_parse_flag_minus1_full() {
        let r = serde_json::json!({"flag": "-1", "code": "1"});
        let (status, _detail) = parse_enroll_result(
            &r,
            &SniperTarget::default(),
        );
        assert_eq!(status, "unavailable");
    }
}
