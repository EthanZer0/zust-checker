//! JWClient — 方正教务 API 封装
//!
//! 对应 Python newjwxt/client.py
//! 包含 13 个 API 方法：用户信息、成绩、课表、考试、选课等
//!
//! IMPORTANT: POST 参数使用 Vec<(&str, &str)> 而非 HashMap，以保持插入顺序。
//! 方正教务服务器对 POST body 的字段顺序敏感。

use crate::error::{Result, ZustError};
use crate::session::HttpSession;
use crate::types::{CourseTab, StudentContext};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

const JW_BASE: &str = "https://newjwxt.zust.edu.cn/jwglxt";

/// JWClient — 教务系统 API 客户端
#[derive(Clone)]
pub struct JWClient {
    pub session: HttpSession,
    pub csrf: String,
    pub main_url: String,
    /// 学生上下文（缓存在首次获取后）
    pub ctx: StudentContext,
}

impl JWClient {
    /// 从 cookies 创建客户端
    pub fn new(cookies: &HashMap<String, String>) -> Result<Self> {
        let session = HttpSession::from_cookies(cookies)?;
        let csrf = cookies.get("_csrf").cloned().unwrap_or_default();
        let main_url = cookies
            .get("_main_url")
            .cloned()
            .unwrap_or_else(|| format!("{JW_BASE}/xtgl/index_initMenu.html"));

        Ok(Self {
            session,
            csrf,
            main_url,
            ctx: StudentContext::default(),
        })
    }

    /// 更新客户端状态（用于 session 刷新后）
    pub fn update_state(&mut self, cookies: &HashMap<String, String>) -> Result<()> {
        self.session = HttpSession::from_cookies(cookies)?;
        self.csrf = cookies.get("_csrf").cloned().unwrap_or_default();
        self.main_url = cookies
            .get("_main_url")
            .cloned()
            .unwrap_or_else(|| format!("{JW_BASE}/xtgl/index_initMenu.html"));
        Ok(())
    }

    /// 从当前 session 创建新的 JWClient
    pub fn clone_session(&self, cookies: &HashMap<String, String>) -> Result<Self> {
        Self::new(cookies)
    }

    // ── Private helpers ──

    /// API 请求标准 headers
    fn api_headers(&self) -> HashMap<String, String> {
        let mut h = HashMap::new();
        h.insert(
            "Accept".into(),
            "application/json, text/javascript, */*; q=0.01".into(),
        );
        h.insert("X-Requested-With".into(), "XMLHttpRequest".into());
        h.insert("Referer".into(), self.main_url.clone());
        h.insert("Csrf-Token".into(), self.csrf.clone());
        h
    }

    /// GET 请求（JSON 响应）
    async fn _get(&self, path: &str) -> Result<JsonValue> {
        let url = format!("{JW_BASE}/{path}");
        let (body, _, _) = self
            .session
            .get_with_headers(&url, &self.api_headers())
            .await?;

        match serde_json::from_str::<JsonValue>(&body) {
            Ok(v) => Ok(v),
            Err(_) => {
                let mut m = serde_json::Map::new();
                m.insert(
                    "_raw".into(),
                    JsonValue::String(body.chars().take(2000).collect()),
                );
                Ok(JsonValue::Object(m))
            }
        }
    }

    /// GET 请求（可能是 JSON 也可能是 HTML）
    async fn _get_html(&self, path: &str) -> Result<JsonValue> {
        let url = format!("{JW_BASE}/{path}");
        let mut headers = self.api_headers();
        headers.insert("Accept".into(), "text/html, */*; q=0.01".into());

        let (body, _, _) = self.session.get_with_headers(&url, &headers).await?;

        // 尝试 JSON 解析
        if let Ok(v) = serde_json::from_str::<JsonValue>(&body) {
            return Ok(v);
        }

        // 返回 HTML
        let mut m = serde_json::Map::new();
        m.insert("_html".into(), JsonValue::String(body));
        Ok(JsonValue::Object(m))
    }

    /// POST 请求（JSON 响应） — 使用 Vec 保序
    async fn _post(&self, path: &str, data: &[(&str, &str)]) -> Result<JsonValue> {
        let url = format!("{JW_BASE}/{path}");

        // Build form body string for debug logging
        let form_body: String = data
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");

        log::info!("POST {url}");
        log::info!("  body ({len} fields): {form_body}", len = data.len(),);

        let (body, final_url, status) = self
            .session
            .post_with_headers(&url, data, &self.api_headers())
            .await?;

        log::info!("  → status={status}, url={final_url}");
        let preview_len = body.floor_char_boundary(body.len().min(500));
        log::info!("  → body preview: {}", &body[..preview_len]);

        match serde_json::from_str::<JsonValue>(&body) {
            Ok(v) => {
                log::info!("  → parsed as JSON");
                Ok(v)
            }
            Err(e) => {
                log::warn!("  → JSON parse failed: {e}");
                let mut m = serde_json::Map::new();
                m.insert(
                    "_raw".into(),
                    JsonValue::String(body.chars().take(2000).collect()),
                );
                Ok(JsonValue::Object(m))
            }
        }
    }

    /// POST 请求（HTML 响应） — 使用 Vec 保序
    async fn _post_html(&self, path: &str, data: &[(&str, &str)]) -> Result<JsonValue> {
        let url = format!("{JW_BASE}/{path}");
        let mut headers = self.api_headers();
        headers.insert("Accept".into(), "text/html, */*; q=0.01".into());

        let form_body: String = data
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");

        log::info!("POST(HTML) {url}");
        log::info!("  body ({len} fields): {form_body}", len = data.len(),);

        let (body, final_url, status) =
            self.session.post_with_headers(&url, data, &headers).await?;

        log::info!("  → status={status}, url={final_url}");
        let preview_len = body.floor_char_boundary(body.len().min(500));
        log::info!("  → body preview: {}", &body[..preview_len]);

        // 判断是 JSON 还是 HTML
        let trimmed = body.trim_start();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            match serde_json::from_str::<JsonValue>(&body) {
                Ok(v) => {
                    log::info!("  → parsed as JSON");
                    return Ok(v);
                }
                Err(e) => {
                    log::warn!("  → JSON parse failed: {e}");
                }
            }
        }

        let mut m = serde_json::Map::new();
        m.insert("_html".into(), JsonValue::String(body));
        Ok(JsonValue::Object(m))
    }

    // ── Public API Methods ──

    /// 测试 session 是否有效
    pub async fn test_session(&self) -> bool {
        match self
            ._get("xsxxxggl/xsxxwh_cxCkDgxsxx.html?gnmkdm=N100801")
            .await
        {
            Ok(v) => {
                if v.get("_raw").is_some() {
                    v["_raw"].as_str().map_or(false, |s| s.contains("bh_id"))
                } else {
                    true
                }
            }
            Err(_) => false,
        }
    }

    /// 获取用户信息
    pub async fn get_user_info(&self) -> Result<JsonValue> {
        self._get("xsxxxggl/xsxxwh_cxCkDgxsxx.html?gnmkdm=N100801")
            .await
    }

    /// 获取成绩
    pub async fn get_grades(&self, year: &str, term: &str) -> Result<JsonValue> {
        let data: Vec<(&str, &str)> = vec![
            ("xnm", year),
            ("xqm", term),
            ("queryModel.showCount", "500"),
            ("queryModel.currentPage", "1"),
            ("queryModel.sortName", ""),
            ("queryModel.sortOrder", "asc"),
        ];

        log::info!("get_grades year={year} term={term}");

        self._post("cjcx/cjcx_cxDgXscj.html?doType=query&gnmkdm=N305005", &data)
            .await
    }

    /// 获取课表
    pub async fn get_schedule(&self, year: &str, term: &str) -> Result<JsonValue> {
        let data: Vec<(&str, &str)> = vec![("xnm", year), ("xqm", term)];

        self._post("kbcx/xskbcx_cxXsKb.html?gnmkdm=N2151", &data)
            .await
    }

    /// 获取考试安排
    pub async fn get_exams(&self, year: &str, term: &str) -> Result<JsonValue> {
        let data: Vec<(&str, &str)> = vec![
            ("xnm", year),
            ("xqm", term),
            ("queryModel.showCount", "100"),
            ("queryModel.currentPage", "1"),
        ];

        self._post(
            "kwgl/kscx_cxXsksxxIndex.html?doType=query&gnmkdm=N358105",
            &data,
        )
        .await
    }

    /// 获取学业情况
    pub async fn get_academics(&self) -> Result<JsonValue> {
        self._get("xsxy/xsxyqk_cxXsxyqkIndex.html?gnmkdm=N105515&layout=default")
            .await
    }

    /// 获取选课首页 → 返回 (HTML, tabs 列表)
    pub async fn xsxk_index(&self) -> Result<(String, Vec<CourseTab>)> {
        let url =
            format!("{JW_BASE}/xsxk/zzxkyzb_cxZzxkYzbIndex.html?gnmkdm=N253512&layout=default");
        let mut headers = self.api_headers();
        headers.insert("Referer".into(), self.main_url.clone());
        headers.insert(
            "User-Agent".into(),
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/142.0.0.0 Safari/537.36"
                .into(),
        );

        let (html, _final_url, _status) = self.session.get_with_headers(&url, &headers).await?;

        // Parse tabs — same pattern as Python _xsxk_index():
        // <a onclick="queryCourse(this, '01', 'A1B2...', '2025', '1024')">主修课程</a>
        let tab_pattern = regex::Regex::new(
            r#"<a[^>]*onclick\s*=\s*"queryCourse\s*\([^,]*,\s*'(\d+)'\s*,\s*'([A-F0-9]{30,})'\s*,\s*'(\d+)'\s*,\s*'(\d+)'\)"[^>]*>([^<]*)</a>"#,
        )
        .map_err(|e| ZustError::Parse(e.to_string()))?;

        let tabs: Vec<CourseTab> = tab_pattern
            .captures_iter(&html)
            .map(|c| CourseTab {
                kklxdm: c.get(1).map(|m| m.as_str().to_string()).unwrap_or_default(),
                xkkz_id: c.get(2).map(|m| m.as_str().to_string()).unwrap_or_default(),
                njdm_id: c.get(3).map(|m| m.as_str().to_string()).unwrap_or_default(),
                zyh_id: c.get(4).map(|m| m.as_str().to_string()).unwrap_or_default(),
                name: c
                    .get(5)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default(),
            })
            .collect();

        log::info!("xsxk_index: found {} course tabs", tabs.len());
        Ok((html, tabs))
    }

    /// 获取学生上下文（jx0502zbid=... 等字段）
    /// HTML 中这些字段是 hidden <input> 标签，如：
    ///   <input type="hidden" name="bh_id" id="bh_id" value="12510252"/>
    pub fn get_student_context(&self, html: &str) -> StudentContext {
        let mut ctx = StudentContext::default();

        // bh_id — <input name="bh_id" ... value="12510252"/>
        if let Some(re) = regex::Regex::new(r#"name="bh_id"[^>]+value="(\d+)""#).ok() {
            if let Some(cap) = re.captures(html) {
                ctx.bh_id = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
            }
        }

        // njdm_id — <input name="njdm_id" ... value="2025"/>
        if let Some(re) = regex::Regex::new(r#"name="njdm_id"[^>]+value="(\d+)""#).ok() {
            if let Some(cap) = re.captures(html) {
                ctx.njdm_id = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
            }
        }

        // zyh_id — <input name="zyh_id" ... value="1024"/>
        if let Some(re) = regex::Regex::new(r#"name="zyh_id"[^>]+value="(\d+)""#).ok() {
            if let Some(cap) = re.captures(html) {
                ctx.zyh_id = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
            }
        }

        // jg_id (hex) — <input name="jg_id" ... value="0A1B2C..."/>
        if let Some(re) = regex::Regex::new(r#"name="jg_id"[^>]+value="([A-Fa-f0-9]+)""#).ok() {
            if let Some(cap) = re.captures(html) {
                ctx.jg_id = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
            }
        }

        // xkxnm (academic year) — <input name="xkxnm" ... value="2026"/>
        if let Some(re) = regex::Regex::new(r#"name="xkxnm"[^>]+value="(\d+)""#).ok() {
            if let Some(cap) = re.captures(html) {
                ctx.xkxnm = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
            }
        }

        // xkxqm (academic term) — <input name="xkxqm" ... value="3"/>
        if let Some(re) = regex::Regex::new(r#"name="xkxqm"[^>]+value="(\d+)""#).ok() {
            if let Some(cap) = re.captures(html) {
                ctx.xkxqm = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
            }
        }

        log::info!(
            "StudentContext: bh_id={}, njdm_id={}, zyh_id={}, jg_id={}, xkxnm={}, xkxqm={}",
            ctx.bh_id,
            ctx.njdm_id,
            ctx.zyh_id,
            ctx.jg_id,
            ctx.xkxnm,
            ctx.xkxqm
        );

        ctx
    }

    /// 获取可选课程（HTML 响应，单页）
    pub async fn get_available_courses(
        &self,
        xkkz_id: &str,
        kklxdm: &str,
        ctx: &StudentContext,
        page: u32,
    ) -> Result<JsonValue> {
        let kspage_s = page.to_string();
        let data: Vec<(&str, &str)> = vec![
            ("xkkz_id", xkkz_id),
            ("xszxzt", "1"),
            ("kklxdm", kklxdm),
            ("njdm_id", &ctx.njdm_id),
            ("zyh_id", &ctx.zyh_id),
            ("kspage", &kspage_s),
            ("jspage", "0"),
        ];

        self._post_html("xsxk/zzxkyzb_cxZzxkYzbDisplay.html?gnmkdm=N253512", &data)
            .await
    }

    /// 获取课程列表（分页 JSON）— 字段顺序与 Python 严格一致
    pub async fn get_part_display(
        &self,
        xkkz_id: &str,
        kklxdm: &str,
        ctx: &StudentContext,
        page: u32,
        page_size: u32,
    ) -> Result<JsonValue> {
        let njdm = &ctx.njdm_id;
        let zyh = &ctx.zyh_id;
        let bh = &ctx.bh_id;
        let jg = &ctx.jg_id;
        let xnm = &ctx.xkxnm;
        let xqm = &ctx.xkxqm;

        // kspage = page (row index), jspage = page_size, NOT fixed 10
        let kspage = page.to_string();
        let jspage = page_size.to_string();

        // Fields in EXACT order matching Python get_part_display
        let data: Vec<(&str, &str)> = vec![
            ("rwlx", "2"),
            ("xklc", "1"),
            ("xkly", "0"),
            ("bklx_id", "0"),
            ("sfkkjyxdxnxq", "0"),
            ("kzkcgs", "0"),
            ("xqh_id", "1"),
            ("jg_id", jg),
            ("njdm_id_1", njdm),
            ("zyh_id_1", zyh),
            ("gnjkxdnj", "0"),
            ("zyh_id", zyh),
            ("zyfx_id", "wfx"),
            ("njdm_id", njdm),
            ("bh_id", bh),
            ("bjgkczxbbjwcx", "0"),
            ("xbm", "1"),
            ("xslbdm", "421"),
            ("mzm", "01"),
            ("xz", "4"),
            ("ccdm", "3"),
            ("xsbj", "0"),
            ("sfkknj", "0"),
            ("sfkkzy", "0"),
            ("kzybkxy", "0"),
            ("sfznkx", "0"),
            ("zdkxms", "0"),
            ("sfkxq", "0"),
            ("bhbcyxkjxb", "0"),
            ("sfkcfx", "0"),
            ("kkbk", "0"),
            ("kkbkdj", "0"),
            ("bklbkcj", "0"),
            ("sfkgbcx", "0"),
            ("sfrxtgkcxd", "0"),
            ("tykczgxdcs", "0"),
            ("xkxnm", xnm),
            ("xkxqm", xqm),
            ("kklxdm", kklxdm),
            ("bbhzxjxb", "0"),
            ("zxgbxkkg", "0"),
            ("xkkz_id", xkkz_id),
            ("rlkz", "0"),
            ("xkzgbj", "0"),
            ("kspage", &kspage),
            ("jspage", &jspage),
            ("jxbzb", ""),
        ];

        self._post(
            "xsxk/zzxkyzb_cxZzxkYzbPartDisplay.html?gnmkdm=N253512",
            &data,
        )
        .await
    }

    /// 获取所有课程（自动翻页，每页 70 条，最多 500 条以防无限循环）
    pub async fn get_all_courses(
        &self,
        xkkz_id: &str,
        kklxdm: &str,
        ctx: &StudentContext,
    ) -> Result<Vec<JsonValue>> {
        log::info!("get_all_courses: xkkz_id={xkkz_id}, kklxdm={kklxdm}, ctx={ctx:?}");

        let mut all = Vec::new();
        let mut page: u32 = 1;

        loop {
            let page_size = page + 69;
            log::info!("  fetching page {page}..{page_size}");

            let r = self
                .get_part_display(xkkz_id, kklxdm, ctx, page, page_size)
                .await?;

            let items = extract_items(&r);
            log::info!(
                "  → got {} items (raw keys: {:?})",
                items.len(),
                r.as_object().map(|o| o.keys().collect::<Vec<_>>())
            );

            if items.is_empty() {
                log::info!("  → empty page, stopping");
                break;
            }

            all.extend(items);
            page += 70;

            if page > 500 {
                log::warn!("  → hit 500 page limit, stopping");
                break;
            }
        }

        log::info!("get_all_courses: total {} courses", all.len());
        Ok(all)
    }

    /// 获取课程详情（教学班列表）
    pub async fn get_course_detail(
        &self,
        xkkz_id: &str,
        kklxdm: &str,
        kch_id: &str,
        ctx: &StudentContext,
    ) -> Result<JsonValue> {
        let njdm = &ctx.njdm_id;
        let zyh = &ctx.zyh_id;
        let bh = &ctx.bh_id;
        let jg = &ctx.jg_id;
        let xnm = &ctx.xkxnm;
        let xqm = &ctx.xkxqm;

        // Fields in EXACT order matching Python get_course_detail
        let data: Vec<(&str, &str)> = vec![
            ("rwlx", "2"),
            ("xkly", "0"),
            ("bklx_id", "0"),
            ("sfkkjyxdxnxq", "0"),
            ("kzkcgs", "0"),
            ("xqh_id", "1"),
            ("jg_id", jg),
            ("zyh_id", zyh),
            ("zyfx_id", "wfx"),
            ("txbsfrl", "0"),
            ("njdm_id", njdm),
            ("bh_id", bh),
            ("xbm", "1"),
            ("xslbdm", "421"),
            ("mzm", "01"),
            ("xz", "4"),
            ("ccdm", "3"),
            ("xsbj", "0"),
            ("sfkknj", "0"),
            ("gnjkxdnj", "0"),
            ("sfkkzy", "0"),
            ("kzybkxy", "0"),
            ("sfznkx", "0"),
            ("zdkxms", "0"),
            ("sfkxq", "0"),
            ("bhbcyxkjxb", "0"),
            ("sfkcfx", "0"),
            ("bbhzxjxb", "0"),
            ("kkbk", "0"),
            ("kkbkdj", "0"),
            ("bklbkcj", "0"),
            ("xkxnm", xnm),
            ("xkxqm", xqm),
            ("xkxskcgskg", "1"),
            ("rlkz", "0"),
            ("cdrlkz", "0"),
            ("cxcykclxxskg", "0"),
            ("rlzlkz", "1"),
            ("kklxdm", kklxdm),
            ("kch_id", kch_id),
            ("jxbzcxskg", "0"),
            ("zxgbxkkg", "0"),
            ("xklc", "1"),
            ("xkkz_id", xkkz_id),
            ("cxbj", "0"),
            ("fxbj", "0"),
        ];

        self._post(
            "xsxk/zzxkyzbjk_cxJxbWithKchZzxkYzb.html?gnmkdm=N253512",
            &data,
        )
        .await
    }

    /// 获取已选课程
    /// Parameters match the browser's actual request (NOT the Python implementation's guess)
    pub async fn get_selected_courses(&self, ctx: &StudentContext) -> Result<JsonValue> {
        // Browser sends: jg_id, zyh_id, njdm_id, zyfx_id, bh_id, xz, ccdm, xqh_id, xkxnm, xkxqm, xkly
        let data: Vec<(&str, &str)> = vec![
            ("jg_id", &ctx.jg_id),
            ("zyh_id", &ctx.zyh_id),
            ("njdm_id", &ctx.njdm_id),
            ("zyfx_id", "wfx"),
            ("bh_id", &ctx.bh_id),
            ("xz", "4"),
            ("ccdm", "3"),
            ("xqh_id", "1"),
            ("xkxnm", &ctx.xkxnm),
            ("xkxqm", &ctx.xkxqm),
            ("xkly", "1"),
        ];

        let result = self
            ._post(
                "xsxk/zzxkyzb_cxZzxkYzbChoosedDisplay.html?gnmkdm=N253512",
                &data,
            )
            .await?;

        log::info!(
            "get_selected_courses result: is_array={}, len={}",
            result.is_array(),
            result.as_array().map(|a| a.len()).unwrap_or(0)
        );

        Ok(result)
    }

    /// 选课（抢课核心）
    pub async fn enroll_course(
        &self,
        jxb_ids: &str,
        kch_id: &str,
        kcmc: &str,
        xkkz_id: &str,
        kklxdm: &str,
        xkxnm: &str,
        xkxqm: &str,
        njdm_id: &str,
        zyh_id: &str,
    ) -> Result<JsonValue> {
        let data: Vec<(&str, &str)> = vec![
            ("jxb_ids", jxb_ids),
            ("kch_id", kch_id),
            ("kcmc", kcmc),
            ("rwlx", "2"),
            ("rlkz", "0"),
            ("cdrlkz", "0"),
            ("rlzlkz", "1"),
            ("sxbj", "1"),
            ("xxkbj", "0"),
            ("qz", "0"),
            ("cxbj", "0"),
            ("xkkz_id", xkkz_id),
            ("njdm_id", njdm_id),
            ("zyh_id", zyh_id),
            ("kklxdm", kklxdm),
            ("xklc", "1"),
            ("xkxnm", xkxnm),
            ("xkxqm", xkxqm),
            ("jcxx_id", ""),
        ];

        self._post("xsxk/zzxkyzbjk_xkBcZyZzxkYzb.html?gnmkdm=N253512", &data)
            .await
    }

    /// 退课
    pub async fn drop_course(
        &self,
        jxb_ids: &str,
        kch_id: &str,
        xkxnm: &str,
        xkxqm: &str,
    ) -> Result<JsonValue> {
        let data: Vec<(&str, &str)> = vec![
            ("kch_id", kch_id),
            ("jxb_ids", jxb_ids),
            ("xkxnm", xkxnm),
            ("xkxqm", xkxqm),
            ("txbsfrl", "0"),
        ];

        self._post("xsxk/zzxkyzb_tuikBcZzxkYzb.html?gnmkdm=N253512", &data)
            .await
    }

    /// 预检测课程（选课前的容量检查）
    pub async fn precheck_course(
        &self,
        jxb_ids: &str,
        kch_id: &str,
        kklxdm: &str,
        xkxnm: &str,
        xkxqm: &str,
        njdm_id: &str,
        zyh_id: &str,
    ) -> Result<JsonValue> {
        let data: Vec<(&str, &str)> = vec![
            ("jxb_ids", jxb_ids),
            ("xkxnm", xkxnm),
            ("xkxqm", xkxqm),
            ("bj", "7"),
            ("kch_id", kch_id),
            ("njdm_id", njdm_id),
            ("zyh_id", zyh_id),
            ("kklxdm", kklxdm),
        ];

        self._post("xsxk/zzxkyzb_cxXkTitleMsg.html?gnmkdm=N253512", &data)
            .await
    }
}

/// 从 JSON 响应中提取 items 数组
fn extract_items(v: &JsonValue) -> Vec<JsonValue> {
    if let Some(arr) = v.get("items").and_then(|a| a.as_array()) {
        return arr.clone();
    }
    if let Some(arr) = v.get("rows").and_then(|a| a.as_array()) {
        return arr.clone();
    }
    if let Some(arr) = v.get("tmpList").and_then(|a| a.as_array()) {
        return arr.clone();
    }
    Vec::new()
}
