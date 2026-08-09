//! 领域类型定义
//! 所有类型均派生 Serialize/Deserialize 以支持 Tauri IPC 传输

use serde::{Deserialize, Serialize};

/// 用户信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserInfo {
    pub name: String,
    pub student_id: String,
    pub department: String,
    #[serde(default)]
    pub raw: serde_json::Value,
}

/// 成绩条目
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GradeEntry {
    pub name: String,
    pub score: String,
    pub credit: String,
    pub point: String,
    pub nature: String,
    pub course_type: String,
    pub term_name: String,
    pub year: String,
    pub term: String,
    #[serde(default)]
    pub remark: String,
}

/// 学期分组
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SemesterGroup {
    pub key: String,
    pub year: String,
    pub term: String,
    pub term_name: String,
    pub courses: Vec<GradeEntry>,
    pub gpa: f64,
}

/// 成绩汇总（Dashboard 用）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GradeSummary {
    pub courses: Vec<GradeEntry>,
    pub semesters: Vec<SemesterGroup>,
    pub gpa: f64,
    pub total_credits: f64,
}

/// 课表条目
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScheduleEntry {
    pub day: String,
    pub sessions: String,
    pub course_name: String,
    pub teacher: String,
    pub location: String,
    pub weeks: String,
}

/// 考试条目
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExamEntry {
    pub course_name: String,
    pub datetime: String,
    pub location: String,
    pub seat: String,
}

/// 选课板块（Tab）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CourseTab {
    pub kklxdm: String,
    pub xkkz_id: String,
    pub njdm_id: String,
    pub zyh_id: String,
    pub name: String,
}

/// 课程列表项
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CourseListItem {
    pub index: usize,
    pub name: String,
    pub credits: String,
    pub course_type: String,
    pub kch_id: String,
}

/// 选课上下文（从主页 HTML 提取）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StudentContext {
    pub bh_id: String,
    pub njdm_id: String,
    pub zyh_id: String,
    pub jg_id: String,
    pub xkxnm: String,
    pub xkxqm: String,
}

/// 抢课目标
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SniperTarget {
    pub jxb_ids: String,
    pub kch_id: String,
    pub kcmc: String,
    pub xkkz_id: String,
    pub kklxdm: String,
    pub xkxnm: String,
    pub xkxqm: String,
    pub njdm_id: String,
    pub zyh_id: String,
}

/// 抢课单轮结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperTickResult {
    pub targets: Vec<SniperTargetStatus>,
    pub total_attempts: u64,
    pub elapsed_secs: f64,
    /// 本轮是否检测到 session 仍有效（false 表示可能需重登）
    pub session_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperTargetStatus {
    pub kch_id: String,
    pub kcmc: String,
    pub status: String,
    pub detail: String,
}

/// 抢课统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperStats {
    pub kch_id: String,
    pub kcmc: String,
    pub attempts: u64,
    pub last_status: String,
    pub last_detail: String,
}

/// 抢课状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniperStatusInfo {
    pub running: bool,
    pub targets: Vec<SniperTarget>,
    pub stats: Vec<SniperStats>,
    pub total_attempts: u64,
    pub elapsed_secs: f64,
    pub interval_ms: u64,
}

/// Session 文件格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub cookies: std::collections::HashMap<String, String>,
    pub saved_at: f64,
}

/// 教学班详情条目（用于前端展示）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeachingClass {
    pub index: usize,
    pub teacher: String,
    pub time: String,
    pub location: String,
    pub enrolled: String,
    pub capacity: String,
    pub campus: String,
    pub jxb_id: String,
}

/// 课程详情（去重后的课程 + 教学班列表）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CourseDetail {
    pub course_name: String,
    pub kch_id: String,
    pub teaching_classes: Vec<TeachingClass>,
}

/// Dashboard 数据（一次返回用户信息+成绩）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DashboardData {
    pub user: UserInfo,
    pub grades: GradeSummary,
}
