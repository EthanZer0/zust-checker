// Type definitions mirroring Rust zust-core types

export interface UserInfo {
  name: string;
  student_id: string;
  department: string;
  raw?: any;
}

export interface GradeEntry {
  name: string;
  score: string;
  credit: string;
  point: string;
  nature: string;
  course_type: string;
  term_name: string;
  year: string;
  term: string;
  remark: string;
}

export interface SemesterGroup {
  key: string;
  year: string;
  term: string;
  term_name: string;
  courses: GradeEntry[];
  gpa: number;
}

export interface GradeSummary {
  courses: GradeEntry[];
  semesters: SemesterGroup[];
  gpa: number;
  total_credits: number;
}

export interface DashboardData {
  user: UserInfo;
  grades: GradeSummary;
}

export interface ScheduleEntry {
  day: string;
  sessions: string;
  course_name: string;
  teacher: string;
  location: string;
  weeks: string;
}

export interface ExamEntry {
  course_name: string;
  datetime: string;
  location: string;
  seat: string;
}

export interface CourseTab {
  kklxdm: string;
  xkkz_id: string;
  njdm_id: string;
  zyh_id: string;
  name: string;
}

export interface SniperTarget {
  jxb_ids: string;
  kch_id: string;
  kcmc: string;
  xkkz_id: string;
  kklxdm: string;
  xkxnm: string;
  xkxqm: string;
  njdm_id: string;
  zyh_id: string;
}

export interface SniperTargetStatus {
  kch_id: string;
  kcmc: string;
  status: string;
  detail: string;
}

export interface SniperTickResult {
  targets: SniperTargetStatus[];
  total_attempts: number;
  elapsed_secs: number;
  /** false means the session may have expired → frontend should prompt re-login */
  session_valid?: boolean;
}

export interface SniperStats {
  kch_id: string;
  kcmc: string;
  attempts: number;
  last_status: string;
  last_detail: string;
}

export interface SniperStatusInfo {
  running: boolean;
  targets: SniperTarget[];
  stats: SniperStats[];
  total_attempts: number;
  elapsed_secs: number;
  interval_ms: number;
}

export interface LoginStep {
  step: number;
  message: string;
}
