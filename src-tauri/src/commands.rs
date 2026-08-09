//! Tauri Commands — 前端 ↔ 后端 IPC 桥接
//!
//! IMPORTANT: 所有 `state.client.lock()` 必须在 `.await` 之前 drop。
//! MutexGuard 不是 Send，不能跨越 await 点。
//! 模式：clone JWClient → drop MutexGuard → await on clone

use crate::state::{AppState, SniperState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
use zust_core::auth::AuthManager;
use zust_core::client::JWClient;
use zust_core::sniper::EnrollSniper;
use zust_core::types::*;

// ═══════════════════════════════════════════════════════════════════════════
// 认证命令
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn check_session(
    state: State<'_, AppState>,
) -> Result<Option<UserInfo>, String> {
    let cookies = {
        let auth = state.auth_manager.lock().map_err(|e| e.to_string())?;
        auth.load_session()
    };

    let Some(cookies) = cookies else {
        return Ok(None);
    };

    let client = JWClient::new(&cookies).map_err(|e| e.to_string())?;
    if !client.test_session().await {
        return Ok(None);
    }

    let raw = client.get_user_info().await.map_err(|e| e.to_string())?;
    if raw.get("_raw").is_some() {
        return Ok(None);
    }

    let user = parse_user_info(&raw);

    let mut lock = state.client.lock().map_err(|e| e.to_string())?;
    *lock = Some(client);

    Ok(Some(user))
}

#[tauri::command]
pub async fn login(
    app: AppHandle,
    state: State<'_, AppState>,
    username: String,
    cas_password: String,
    wisedu_password: String,
) -> Result<UserInfo, String> {
    let auth = AuthManager::new();

    let app_handle = app.clone();
    let cookies = auth
        .full_login(&username, &cas_password, &wisedu_password, move |step| {
            let _ = app_handle.emit("login-step", &step);
        })
        .await
        .map_err(|e| e.to_string())?;

    let client = JWClient::new(&cookies).map_err(|e| e.to_string())?;
    let raw = client.get_user_info().await.map_err(|e| e.to_string())?;
    let user = parse_user_info(&raw);

    let mut lock = state.client.lock().map_err(|e| e.to_string())?;
    *lock = Some(client);

    // 登录成功后保存凭据
    save_credentials_to_file(&state, &username, &cas_password, &wisedu_password);

    Ok(user)
}

/// 从文件加载缓存的凭据
fn load_credentials_from_file(state: &AppState) -> Option<(String, String, String)> {
    let path = state.config_dir.join("credentials.json");
    if !path.exists() { return None; }
    let content = std::fs::read_to_string(&path).ok()?;
    let v: serde_json::Value = serde_json::from_str(&content).ok()?;
    Some((
        v.get("username")?.as_str()?.to_string(),
        v.get("cas_password")?.as_str()?.to_string(),
        v.get("wisedu_password")?.as_str()?.to_string(),
    ))
}

/// 保存凭据到文件
fn save_credentials_to_file(state: &AppState, username: &str, cas_password: &str, wisedu_password: &str) {
    let path = state.config_dir.join("credentials.json");
    let json = serde_json::json!({
        "username": username,
        "cas_password": cas_password,
        "wisedu_password": wisedu_password,
    }).to_string();
    let _ = std::fs::create_dir_all(&state.config_dir);
    let _ = std::fs::write(&path, &json);
}

#[tauri::command]
pub fn load_credentials(
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    if let Some((u, c, w)) = load_credentials_from_file(&state) {
        Ok(Some(serde_json::json!({
            "username": u,
            "cas_password": c,
            "wisedu_password": w,
        })))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    {
        let auth = state.auth_manager.lock().map_err(|e| e.to_string())?;
        auth.clear_session().map_err(|e| e.to_string())?;
    }
    let mut lock = state.client.lock().map_err(|e| e.to_string())?;
    *lock = None;
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// 数据查询命令
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn refresh_dashboard(
    state: State<'_, AppState>,
) -> Result<DashboardData, String> {
    let client = get_client(&state)?;
    let raw_user = client.get_user_info().await.map_err(|e| e.to_string())?;
    let raw_grades = client.get_grades("", "").await.map_err(|e| e.to_string())?;

    Ok(DashboardData {
        user: parse_user_info(&raw_user),
        grades: parse_grades(&raw_grades),
    })
}

#[tauri::command]
pub async fn get_grades(
    state: State<'_, AppState>,
    year: Option<String>,
    term: Option<String>,
) -> Result<GradeSummary, String> {
    let client = get_client(&state)?;
    let raw = client
        .get_grades(&year.unwrap_or_default(), &term.unwrap_or_default())
        .await
        .map_err(|e| e.to_string())?;
    Ok(parse_grades(&raw))
}

#[tauri::command]
pub async fn get_schedule(
    state: State<'_, AppState>,
    year: String,
    term: String,
) -> Result<Vec<ScheduleEntry>, String> {
    let client = get_client(&state)?;
    let raw = client.get_schedule(&year, &term).await.map_err(|e| e.to_string())?;
    Ok(parse_schedule(&raw))
}

#[tauri::command]
pub async fn get_exams(
    state: State<'_, AppState>,
    year: String,
    term: String,
) -> Result<Vec<ExamEntry>, String> {
    let client = get_client(&state)?;
    let raw = client.get_exams(&year, &term).await.map_err(|e| e.to_string())?;
    Ok(parse_exams(&raw))
}

// ═══════════════════════════════════════════════════════════════════════════
// 选课命令
// ═══════════════════════════════════════════════════════════════════════════

#[tauri::command]
pub async fn get_course_tabs(
    state: State<'_, AppState>,
) -> Result<Vec<CourseTab>, String> {
    let client = get_client(&state)?;
    let (_, tabs) = client.xsxk_index().await.map_err(|e| e.to_string())?;
    Ok(tabs)
}

#[tauri::command]
pub async fn get_course_list(
    state: State<'_, AppState>,
    xkkz_id: String,
    kklxdm: String,
) -> Result<Vec<serde_json::Value>, String> {
    let client = get_client(&state)?;
    let (html, _) = client.xsxk_index().await.map_err(|e| e.to_string())?;
    let ctx = client.get_student_context(&html);
    client
        .get_all_courses(&xkkz_id, &kklxdm, &ctx)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_course_detail(
    state: State<'_, AppState>,
    xkkz_id: String,
    kklxdm: String,
    kch_id: String,
) -> Result<serde_json::Value, String> {
    let client = get_client(&state)?;
    let (html, _) = client.xsxk_index().await.map_err(|e| e.to_string())?;
    let ctx = client.get_student_context(&html);
    client
        .get_course_detail(&xkkz_id, &kklxdm, &kch_id, &ctx)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_selected_courses(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let client = get_client(&state)?;
    let (html, _) = client.xsxk_index().await.map_err(|e| e.to_string())?;
    let ctx = client.get_student_context(&html);
    client
        .get_selected_courses(&ctx)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn enroll_single(
    state: State<'_, AppState>,
    target: SniperTarget,
) -> Result<serde_json::Value, String> {
    let client = get_client(&state)?;
    client
        .enroll_course(
            &target.jxb_ids, &target.kch_id, &target.kcmc,
            &target.xkkz_id, &target.kklxdm,
            &target.xkxnm, &target.xkxqm,
            &target.njdm_id, &target.zyh_id,
        )
        .await
        .map_err(|e| e.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════
// 抢课命令
// ═══════════════════════════════════════════════════════════════════════════

/// 尝试用保存的凭据重新登录，更新 AppState.client 与 EnrollSniper
async fn try_relogin(
    app: &AppHandle,
    config_dir: &std::path::Path,
    app_client: &Arc<Mutex<Option<JWClient>>>,
    sniper: &Arc<EnrollSniper>,
) -> Result<(), String> {
    // 1. 从文件加载凭据
    let creds_path = config_dir.join("credentials.json");
    if !creds_path.exists() {
        return Err("credentials.json not found".into());
    }
    let content = std::fs::read_to_string(&creds_path).map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let username = v.get("username").and_then(|s| s.as_str()).ok_or("no username in creds")?;
    let cas_pw = v.get("cas_password").and_then(|s| s.as_str()).unwrap_or("");
    let wisedu_pw = v.get("wisedu_password").and_then(|s| s.as_str()).unwrap_or(cas_pw);

    log::info!("try_relogin: re-logging as {username}");

    // 2. 完整登录
    let auth = AuthManager::new();
    let app_h = app.clone();
    let cookies = auth
        .full_login(username, cas_pw, wisedu_pw, move |step| {
            let _ = app_h.emit("login-step", &step);
        })
        .await
        .map_err(|e| e.to_string())?;

    // 3. 创建新 client
    let new_client = JWClient::new(&cookies).map_err(|e| e.to_string())?;

    // 4. 更新 AppState client
    {
        let mut lock = app_client.lock().map_err(|e| e.to_string())?;
        *lock = Some(new_client.clone());
    }

    // 5. 更新 EnrollSniper 内部的 client（通过 Mutex）
    sniper.update_client(new_client);

    Ok(())
}

#[tauri::command]
pub async fn sniper_start(
    app: AppHandle,
    sniper_state: State<'_, SniperState>,
    state: State<'_, AppState>,
    targets: Vec<SniperTarget>,
    interval_ms: u64,
    stagger_ms: u64,
) -> Result<(), String> {
    // Stop previous sniper
    {
        let lock = sniper_state.sniper.lock().map_err(|e| e.to_string())?;
        if let Some(s) = lock.as_ref() {
            s.set_running(false);
        }
    }
    {
        let mut lock = sniper_state.handle.lock().map_err(|e| e.to_string())?;
        if let Some(h) = lock.take() {
            h.abort();
        }
    }

    let client = get_client(&state)?;
    let mut sniper = EnrollSniper::new(client);
    sniper.set_targets(targets.clone());
    sniper.set_running(true);
    sniper.reset();

    let sniper_arc = Arc::new(sniper);
    let sniper_clone = sniper_arc.clone();

    *sniper_state.sniper.lock().map_err(|e| e.to_string())? = Some(sniper_arc);

    // 捕获凭据和 AppState client 指针用于自动重登
    let config_dir = state.config_dir.clone();
    let app_client = Arc::clone(&state.client);

    let handle = tokio::spawn(async move {
        let base_interval_ms = interval_ms;
        let stagger_ms = stagger_ms;
        let mut consecutive_errors: u32 = 0;

        loop {
            if !sniper_clone.is_running() {
                break;
            }
            match sniper_clone.tick(stagger_ms).await {
                Ok(result) => {
                    // ── 连续成功 → 退避归零 ──
                    consecutive_errors = 0;

                    // ── Session 失效检测 ──
                    if !result.session_valid
                        || result.targets.iter().any(|t| t.status == "session_expired")
                    {
                        log::warn!("sniper: session expired, attempting auto re-login...");
                        let _ = app.emit("sniper-reconnecting", &serde_json::json!({}));

                        match try_relogin(&app, &config_dir, &app_client, &sniper_clone).await {
                            Ok(()) => {
                                log::info!("sniper: re-login succeeded, resuming");
                                let _ = app.emit("sniper-reconnected", &serde_json::json!({}));
                                consecutive_errors = 0;
                                continue; // 直接下一轮抢课
                            }
                            Err(e) => {
                                log::error!("sniper: re-login failed: {e}");
                                let _ = app.emit("sniper-tick", &result);
                                let _ = app.emit(
                                    "sniper-session-expired",
                                    &serde_json::json!({"detail": format!("重登失败: {e}")}),
                                );
                                sniper_clone.set_running(false);
                                break;
                            }
                        }
                    }

                    let has_success = result.targets.iter().any(|t| t.status == "success");
                    let _ = app.emit("sniper-tick", &result);
                    if has_success {
                        let _ = app.emit("sniper-success", &result);
                        sniper_clone.set_running(false);
                        break;
                    }
                }
                Err(e) => {
                    consecutive_errors += 1;
                    let _ = app.emit("sniper-error", &e.to_string());
                }
            }

            // ── 间隔 = base + jitter(±20%) + 错误指数退避 ──
            let mut delay_ms = base_interval_ms as f64;

            // 随机抖动 ±20%
            let jitter_factor = 1.0 + (sniper_clone.total_attempts() as f64 % 20.0 - 10.0) / 50.0; // ±20%
            delay_ms *= jitter_factor;

            // 指数退避：2^(consecutive_errors-1) × 500ms，上限 interval×4 或 5000ms
            if consecutive_errors > 0 {
                let backoff = 500u64 * (1u64 << (consecutive_errors - 1));
                let max_backoff = (base_interval_ms * 4).max(5000);
                delay_ms += backoff.min(max_backoff) as f64;
            }

            let delay = tokio::time::Duration::from_millis(delay_ms.max(200.0) as u64);
            log::info!(
                "sniper: sleeping {:?} (base={}ms, errors={}, jitter={:.0}ms)",
                delay, base_interval_ms, consecutive_errors, delay_ms
            );
            tokio::time::sleep(delay).await;
        }
        let _ = app.emit("sniper-stopped", &());
    });

    *sniper_state.handle.lock().map_err(|e| e.to_string())? = Some(handle);
    Ok(())
}

#[tauri::command]
pub async fn sniper_tick(
    sniper_state: State<'_, SniperState>,
) -> Result<SniperTickResult, String> {
    let sniper = {
        let lock = sniper_state.sniper.lock().map_err(|e| e.to_string())?;
        lock.as_ref().ok_or("Sniper not running")?.clone()
    };
    sniper.tick(0).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sniper_stop(
    sniper_state: State<'_, SniperState>,
) -> Result<(), String> {
    {
        let lock = sniper_state.sniper.lock().map_err(|e| e.to_string())?;
        if let Some(s) = lock.as_ref() {
            s.set_running(false);
        }
    }
    {
        let mut lock = sniper_state.handle.lock().map_err(|e| e.to_string())?;
        if let Some(h) = lock.take() {
            h.abort();
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn sniper_status(
    sniper_state: State<'_, SniperState>,
) -> Result<SniperStatusInfo, String> {
    let lock = sniper_state.sniper.lock().map_err(|e| e.to_string())?;
    let sniper = lock.as_ref().ok_or("Sniper not running")?;
    Ok(SniperStatusInfo {
        running: sniper.is_running(),
        targets: sniper.targets().to_vec(),
        stats: sniper.get_stats(),
        total_attempts: sniper.total_attempts(),
        elapsed_secs: sniper.elapsed_secs(),
        interval_ms: 0,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// 辅助函数
// ═══════════════════════════════════════════════════════════════════════════

/// Clone JWClient from state (drops MutexGuard before returning)
fn get_client(state: &AppState) -> Result<JWClient, String> {
    let lock = state.client.lock().map_err(|e| e.to_string())?;
    lock.as_ref().ok_or_else(|| "未登录".to_string()).cloned()
}

// ── 数据解析 ──

fn parse_user_info(raw: &serde_json::Value) -> UserInfo {
    UserInfo {
        name: str_val(raw, &["xsxm", "xm"], "-"),
        student_id: str_val(raw, &["xsdm", "xh", "xh_id"], "-"),
        department: raw
            .get("zymc")
            .or_else(|| raw.get("bm"))
            .and_then(|v| {
                if v.is_object() { v.get("mc").and_then(|m| m.as_str()) }
                else { v.as_str() }
            })
            .unwrap_or("-")
            .to_string(),
        raw: raw.clone(),
    }
}

fn parse_grades(raw: &serde_json::Value) -> GradeSummary {
    let items: Vec<&serde_json::Value> = raw
        .get("items").and_then(|a| a.as_array())
        .or_else(|| raw.get("data").and_then(|d| d.get("items").or_else(|| d.get("rows"))).and_then(|a| a.as_array()))
        .or_else(|| raw.get("data").and_then(|d| d.as_array()))
        .map(|a| a.iter().collect())
        .unwrap_or_default();

    let courses: Vec<GradeEntry> = items.iter().map(|c| GradeEntry {
        name:       str_val(c, &["kcmc", "courseName", "name"], ""),
        score:      str_val(c, &["cj", "score", "bfzcj"], ""),
        credit:     str_val(c, &["xf", "credit"], ""),
        point:      str_val(c, &["jd", "point"], ""),
        nature:     str_val(c, &["kcgsmc", "nature"], ""),
        course_type: str_val(c, &["kclbmc", "type"], ""),
        term_name:  str_val(c, &["xqmmc", "termName"], ""),
        year:       str_val(c, &["xnm", "year"], ""),
        term:       str_val(c, &["xqm", "term"], ""),
        remark:     String::new(),
    }).collect();

    let gpa = calc_gpa(&courses);

    let mut sem_map: HashMap<String, Vec<GradeEntry>> = HashMap::new();
    for c in &courses {
        sem_map
            .entry(format!("{}-{}-{}", c.year, c.term, c.term_name))
            .or_default()
            .push(c.clone());
    }

    let mut semesters: Vec<SemesterGroup> = sem_map
        .into_iter()
        .map(|(key, courses)| {
            let parts: Vec<String> = key.split('-').map(|s| s.to_string()).collect();
            let gpa = calc_gpa(&courses);
            SemesterGroup {
                key,
                year: parts.first().cloned().unwrap_or_default(),
                term: parts.get(1).cloned().unwrap_or_default(),
                term_name: parts.get(2).cloned().unwrap_or_default(),
                courses,
                gpa,
            }
        })
        .collect();

    semesters.sort_by(|a, b| b.key.cmp(&a.key));

    let total_credits: f64 = courses.iter().filter_map(|c| c.credit.parse::<f64>().ok()).sum();

    GradeSummary { courses, semesters, gpa, total_credits }
}

fn calc_gpa(courses: &[GradeEntry]) -> f64 {
    let (tp, tc) = courses.iter().fold((0.0, 0.0), |(tp, tc), c| {
        let cr: f64 = c.credit.parse().unwrap_or(0.0);
        let pt: f64 = c.point.parse().unwrap_or(0.0);
        if cr > 0.0 { (tp + pt * cr, tc + cr) } else { (tp, tc) }
    });
    if tc > 0.0 { (tp / tc * 100.0).round() / 100.0 } else { 0.0 }
}

fn parse_schedule(raw: &serde_json::Value) -> Vec<ScheduleEntry> {
    let items: Vec<&serde_json::Value> = raw
        .get("kbList").and_then(|a| a.as_array())
        .or_else(|| raw.get("items").and_then(|a| a.as_array()))
        .map(|a| a.iter().collect())
        .unwrap_or_default();

    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();
    for c in &items {
        if let Some(nested) = c.get("kbList").and_then(|a| a.as_array()) {
            for kb in nested {
                let entry = ScheduleEntry {
                    day:       str_val(kb, &["xqjmc", "day"], ""),
                    sessions:  str_val(kb, &["jcs", "sessions"], ""),
                    course_name: str_val(kb, &["kcmc", "courseName"], ""),
                    teacher:   str_val(kb, &["jsxm", "teacher"], ""),
                    location:  str_val(kb, &["cdmc", "location"], ""),
                    weeks:     str_val(kb, &["zcd", "weeks"], ""),
                };
                let key = format!("{}|{}|{}|{}|{}", entry.day, entry.sessions, entry.course_name, entry.location, entry.weeks);
                if seen.insert(key) {
                    entries.push(entry);
                }
            }
        } else {
            let entry = ScheduleEntry {
                day:       str_val(c, &["xqjmc", "day"], ""),
                sessions:  str_val(c, &["jcs", "sessions"], ""),
                course_name: str_val(c, &["kcmc", "courseName"], ""),
                teacher:   str_val(c, &["jsxm", "teacher"], ""),
                location:  str_val(c, &["cdmc", "location"], ""),
                weeks:     str_val(c, &["zcd", "weeks"], ""),
            };
            let key = format!("{}|{}|{}|{}|{}", entry.day, entry.sessions, entry.course_name, entry.location, entry.weeks);
            if seen.insert(key) {
                entries.push(entry);
            }
        }
    }
    entries
}

fn parse_exams(raw: &serde_json::Value) -> Vec<ExamEntry> {
    raw.get("items").and_then(|a| a.as_array())
        .map(|a| a.iter().map(|c| ExamEntry {
            course_name: str_val(c, &["kcmc", "courseName"], ""),
            datetime:    str_val(c, &["kssj", "datetime"], ""),
            location:    str_val(c, &["cdmc", "cdbh", "location"], ""),
            seat:        str_val(c, &["zwh", "seat"], ""),
        }).collect())
        .unwrap_or_default()
}

fn str_val(v: &serde_json::Value, keys: &[&str], default: &str) -> String {
    for k in keys {
        if let Some(s) = v.get(k).and_then(|v| v.as_str()) {
            let s = s.trim();
            if !s.is_empty() { return s.to_string(); }
        }
    }
    default.to_string()
}
