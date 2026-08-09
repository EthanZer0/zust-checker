<script lang="ts">
  import {
    getCourseTabs, getCourseList, getCourseDetail,
    getSelectedCourses, sniperStart, sniperStop, sniperStatus,
    currentSemester,
  } from '../lib/api';
  import { listen } from '@tauri-apps/api/event';
  import type { CourseTab, SniperTarget, SniperTickResult, SniperStatusInfo } from '../lib/types';
  import CourseTabSelector from './CourseTabSelector.svelte';
  import CourseList from './CourseList.svelte';
  import CourseDetail from './CourseDetail.svelte';
  import SelectedCourses from './SelectedCourses.svelte';
  import SniperControls from './SniperControls.svelte';
  import Spinner from './Spinner.svelte';

  let tabs = $state<CourseTab[]>([]);
  let activeTabIdx = $state(0);
  let courses = $state<any[]>([]);
  let selectedCourses = $state<any[]>([]);
  let loadingCourses = $state(false);
  let error = $state('');

  // 临时缓存：切换 Tab 回退时无需重新请求
  let coursesCache = $state<Record<string, any[]>>({});
  let selectedCoursesLoaded = $state(false);

  let showDetail = $state(false);
  let detailData = $state<any>(null);
  let detailKchId = $state('');

  // Sniper state
  let sniperRunning = $state(false);
  let sniperTargets = $state<SniperTarget[]>([]);
  let sniperLog = $state<SniperTickResult[]>([]);
  const SNIPER_LOG_MAX = 50;
  let sniperInterval = $state(900);
  function updateInterval(v: number) { sniperInterval = v; }
  let sniperStagger = $state(80);
  function updateStagger(v: number) { sniperStagger = v; }
  let sniperStats = $state<any[]>([]);
  let sniperSuccess = $state(false);
  let sniperSessionExpired = $state(false);
  let sniperReconnecting = $state(false);

  // 当前学期（用于抢课报文的 xkxnm/xkxqm，与课表/登录同一套检测逻辑）
  const sem = currentSemester();

  let mounted = $state(false);
  $effect(() => { mounted = true; });

  async function loadTabs() {
    try {
      tabs = await getCourseTabs();
    } catch (e: any) {
      error = String(e);
    }
  }

  async function loadCourses() {
    if (!tabs[activeTabIdx]) return;
    const t = tabs[activeTabIdx];
    const key = `${t.xkkz_id}:${t.kklxdm}`;

    // 命中缓存则秒出，后台不刷新
    if (coursesCache[key]) {
      courses = coursesCache[key];
      return;
    }

    loadingCourses = true;
    error = '';
    try {
      const [cList, sel] = await Promise.all([
        getCourseList(t.xkkz_id, t.kklxdm),
        selectedCoursesLoaded ? Promise.resolve(null) : getSelectedCourses(),
      ]);
      coursesCache[key] = cList;
      courses = cList;
      if (sel != null) {
        selectedCourses = sel;
        selectedCoursesLoaded = true;
      }
    } catch (e: any) {
      error = String(e);
    } finally {
      loadingCourses = false;
    }
  }

  async function viewDetail(kch_id: string) {
    if (!tabs[activeTabIdx]) return;
    const t = tabs[activeTabIdx];
    try {
      detailData = await getCourseDetail(t.xkkz_id, t.kklxdm, kch_id);
      detailKchId = kch_id;
      showDetail = true;
    } catch (e: any) {
      error = String(e);
    }
  }

  function addTarget(jxb: any) {
    const t = tabs[activeTabIdx];
    if (!t) return;
    const kch = jxb.kch_id || jxb.kch || detailKchId;
    const name = jxb.kcmc || jxb.kch || '';
    const jxbId = jxb.jxb_id || jxb.do_jxb_id || jxb.jxbid || '';
    // 防止重复添加同一个教学班
    if (sniperTargets.some(st => st.jxb_ids === jxbId)) return;
    sniperTargets = [...sniperTargets, {
      jxb_ids: jxbId,
      kch_id: kch,
      kcmc: `(${kch})${name}+-+${jxb.xf || jxb.credit || 0}+学分`,
      xkkz_id: t.xkkz_id,
      kklxdm: t.kklxdm,
      xkxnm: sem.year, xkxqm: sem.term,
      njdm_id: t.njdm_id || '2025',
      zyh_id: t.zyh_id || '1024',
    }];
  }

  function removeTarget(idx: number) {
    sniperTargets = sniperTargets.filter((_, i) => i !== idx);
  }

  async function startSniper() {
    if (sniperTargets.length === 0) return;
    sniperRunning = true;
    sniperLog = [];
    sniperSuccess = false;
    sniperSessionExpired = false;
    sniperReconnecting = false;

    try {
      await sniperStart(sniperTargets, sniperInterval, sniperStagger);
    } catch (e: any) {
      error = String(e);
      sniperRunning = false;
    }
  }

  async function doStopSniper() {
    try {
      await sniperStop();
    } catch (_) {}
    sniperRunning = false;
  }

  // Listen for sniper events
  $effect(() => {
    const u1 = listen<SniperTickResult>('sniper-tick', (e) => {
      sniperLog = [e.payload, ...sniperLog].slice(0, SNIPER_LOG_MAX);
    });
    const u2 = listen('sniper-success', () => {
      sniperSuccess = true;
    });
    const u3 = listen('sniper-stopped', () => {
      sniperRunning = false;
    });
    const u4 = listen('sniper-session-expired', () => {
      sniperSessionExpired = true;
      sniperRunning = false;
    });
    const u5 = listen('sniper-reconnecting', () => {
      sniperReconnecting = true;
    });
    const u6 = listen('sniper-reconnected', () => {
      sniperReconnecting = false;
    });

    return () => {
      u1.then(f => f());
      u2.then(f => f());
      u3.then(f => f());
      u4.then(f => f());
      u5.then(f => f());
      u6.then(f => f());
    };
  });

  $effect(() => { if (mounted) loadTabs(); });
  $effect(() => { if (mounted && tabs.length) loadCourses(); });
</script>

<div class="courses-tab">
  {#if error}
    <div class="status-msg error">{error}</div>
  {/if}

  <CourseTabSelector {tabs} activeTabIdx={activeTabIdx} onSelect={(idx) => { activeTabIdx = idx; loadCourses(); }} />

  {#if sniperTargets.length > 0}
    <SniperControls
      targets={sniperTargets} {removeTarget}
      running={sniperRunning}
      onStart={startSniper} onStop={doStopSniper}
      onIntervalChange={updateInterval}
      onStaggerChange={updateStagger}
      {sniperLog} {sniperSuccess} {sniperSessionExpired} {sniperReconnecting}
      sniperInterval={sniperInterval} sniperStagger={sniperStagger}
    />
  {/if}

  <div class="panels">
    <div class="panel card">
      <h3>可选课程</h3>
      {#if loadingCourses}
        <div class="status-msg loading-inline">
          <Spinner size={22} stroke={3} />
          <span>加载中...</span>
        </div>
      {:else}
        <CourseList {courses} onDetail={viewDetail} />
      {/if}
    </div>

    <div class="panel card">
      <h3>已选课程</h3>
      <SelectedCourses data={selectedCourses} />
    </div>
  </div>

  {#if showDetail && detailData}
    <CourseDetail data={detailData} kchId={detailKchId} onClose={() => showDetail = false} onAddTarget={addTarget} />
  {/if}
</div>

<style>
  .courses-tab { }
  .panels {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    margin-top: 16px;
  }
  .panel {
    padding: 16px 18px;
  }
  .panel:hover {
    transform: none;
    box-shadow: var(--shadow-sm);
  }
  .panel h3 {
    font-size: 13px;
    color: var(--text-secondary);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .status-msg { color: var(--text-secondary); padding: 16px; text-align: center; }
  .status-msg.error { color: var(--error); }
  .loading-inline {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
  }
</style>
