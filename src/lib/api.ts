import { invoke } from '@tauri-apps/api/core';
import type {
  UserInfo, GradeSummary, DashboardData, ScheduleEntry, ExamEntry,
  CourseTab, SniperTarget, SniperTickResult, SniperStatusInfo,
} from './types';

export async function checkSession(): Promise<UserInfo | null> {
  return invoke('check_session');
}

export async function login(username: string, casPassword: string, wiseduPassword: string): Promise<UserInfo> {
  return invoke('login', { username, casPassword: casPassword, wiseduPassword: wiseduPassword });
}

export async function loadCredentials(): Promise<{ username: string; cas_password: string; wisedu_password: string } | null> {
  return invoke('load_credentials');
}

export async function logout(): Promise<void> {
  return invoke('logout');
}

export async function refreshDashboard(): Promise<DashboardData> {
  return invoke('refresh_dashboard');
}

export async function getGrades(year?: string, term?: string): Promise<GradeSummary> {
  return invoke('get_grades', { year: year ?? null, term: term ?? null });
}

export async function getSchedule(year: string, term: string): Promise<ScheduleEntry[]> {
  return invoke('get_schedule', { year, term });
}

export async function getExams(year: string, term: string): Promise<ExamEntry[]> {
  return invoke('get_exams', { year, term });
}

export async function getCourseTabs(): Promise<CourseTab[]> {
  return invoke('get_course_tabs');
}

export async function getCourseList(xkkz_id: string, kklxdm: string): Promise<any[]> {
  return invoke('get_course_list', { xkkzId: xkkz_id, kklxdm });
}

export async function getCourseDetail(xkkz_id: string, kklxdm: string, kch_id: string): Promise<any> {
  return invoke('get_course_detail', { xkkzId: xkkz_id, kklxdm, kchId: kch_id });
}

export async function getSelectedCourses(): Promise<any> {
  return invoke('get_selected_courses');
}

export async function enrollSingle(target: SniperTarget): Promise<any> {
  return invoke('enroll_single', { target });
}

export async function sniperStart(targets: SniperTarget[], intervalMs: number, staggerMs: number): Promise<void> {
  return invoke('sniper_start', { targets, intervalMs, staggerMs });
}

export async function sniperTick(): Promise<SniperTickResult> {
  return invoke('sniper_tick');
}

export async function sniperStop(): Promise<void> {
  return invoke('sniper_stop');
}

export async function sniperStatus(): Promise<SniperStatusInfo> {
  return invoke('sniper_status');
}

/** 根据当前日期推断最新学期 */
export function currentSemester(): { year: string; term: string } {
  const now = new Date();
  const m = now.getMonth() + 1; // 1–12
  const y = now.getFullYear();
  // 上学期(term='3'): 9月–次年1月; 下学期(term='12'): 3月–7月
  if (m >= 9 || m <= 1) {
    return { year: String(y), term: '3' };
  }
  return { year: String(y - 1), term: '12' };
}
