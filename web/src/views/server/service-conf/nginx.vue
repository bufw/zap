<!-- SPDX-License-Identifier: AGPL-3.0-only -->
<template>
  <div class="nginx-page" v-loading="bootLoading">
    <!-- 顶部状态（所有子页共享） -->
    <el-card shadow="never" class="top-card">
      <div class="top-row">
        <div class="title">
          <el-icon :size="18"><Document /></el-icon>
          <span class="t-name">Nginx</span>
          <el-tag v-if="installed" size="small" :type="running ? 'success' : 'danger'">
            {{ running ? t('servicesNginx.running') : t('servicesNginx.notRunning') }}
          </el-tag>
          <el-tag v-if="installed" size="small" type="info">{{ versionText }}</el-tag>
          <el-tag v-if="installed && status.systemd" size="small" type="warning">systemd</el-tag>
        </div>
        <div class="actions" v-if="installed">
          <span class="path mono">{{ status.conf_file }}</span>
          <el-button size="small" :loading="busy" @click="refreshAll">{{
            t('servicesNginx.refreshStatus')
          }}</el-button>
          <el-button
            size="small"
            type="primary"
            plain
            :disabled="!running || busy"
            :loading="acting === 'reload'"
            @click="doControl('reload')"
            >{{ t('servicesNginx.reload') }}</el-button
          >
          <el-button
            size="small"
            type="warning"
            plain
            :disabled="!running || busy"
            :loading="acting === 'restart'"
            @click="doControl('restart')"
            >{{ t('servicesNginx.restart') }}</el-button
          >
          <el-button
            size="small"
            type="success"
            plain
            :disabled="running || busy"
            :loading="acting === 'start'"
            @click="doControl('start')"
            >{{ t('servicesNginx.start') }}</el-button
          >
        </div>
      </div>
    </el-card>

    <!-- 未安装引导 -->
    <el-card v-if="installed === false && !bootLoading" shadow="never" class="mt-3">
      <el-result
        icon="warning"
        :title="t('servicesNginx.notInstalledTitle')"
        :sub-title="t('servicesNginx.notInstalledSub')"
      >
        <template #extra>
          <el-button type="primary" @click="router.push('/appstore')">{{
            t('servicesNginx.goAppstore')
          }}</el-button>
        </template>
      </el-result>
    </el-card>

    <template v-if="installed">
      <!-- 导航：主配置 / 四层转发 / 文件编辑 / WAF 并列一排 -->
      <nav class="nginx-pills">
        <button
          type="button"
          class="nginx-pill"
          :class="{ 'is-active': page === 'conf' }"
          @click="setPage('conf')"
        >
          {{ t('servicesNginx.pageConf') }}
        </button>
        <button
          type="button"
          class="nginx-pill"
          :class="{ 'is-active': page === 'stream' }"
          @click="setPage('stream')"
        >
          {{ t('servicesNginx.pageStream') }}
        </button>
        <button
          type="button"
          class="nginx-pill"
          :class="{ 'is-active': page === 'files' }"
          @click="setPage('files')"
        >
          {{ t('servicesNginx.pageFiles') }}
        </button>
        <button
          type="button"
          class="nginx-pill"
          :class="{ 'is-active': page === 'waf' }"
          @click="setPage('waf')"
        >
          {{ t('servicesNginx.pageWaf') }}
        </button>
      </nav>

      <!-- Nginx 配置：默认站点 + 主配置可视化（替换原“可视化配置” tab） -->
      <div v-show="page === 'conf'" class="nginx-config">
        <el-card shadow="never" class="mt-3 default-vhost">
          <div class="dvh-row">
            <div class="dvh-left">
              <div class="dvh-title">
                {{ t('servicesNginx.defaultVhostTitle') }}
                <el-tag size="small" :type="ipAccess ? 'success' : 'info'">
                  {{ ipAccess ? t('servicesNginx.ipOn') : t('servicesNginx.ipOff') }}
                </el-tag>
              </div>
              <div class="dvh-desc">
                {{ t('servicesNginx.defaultVhostDesc') }}
              </div>
              <div v-if="ipAccess && status.default_page" class="dvh-path mono">
                {{ t('servicesNginx.welcomePage', { path: status.default_page }) }}
              </div>
            </div>
            <div class="dvh-right">
              <el-switch
                v-model="ipAccess"
                :loading="savingDefaultVhost"
                inline-prompt
                :active-text="t('servicesNginx.on')"
                :inactive-text="t('servicesNginx.off')"
                @change="saveDefaultVhost"
              />
            </div>
          </div>
        </el-card>

        <NginxGlobal class="mt-3" />
      </div>

      <StreamConf v-show="page === 'stream'" />

      <!-- 文件编辑 -->
      <div v-show="page === 'files'" class="nginx-config mt-3">
        <div class="editor-layout">
          <div class="file-list">
            <div class="list-head">
              <span>{{ t('servicesNginx.editableConfs') }}</span>
              <el-tag size="small" type="info">{{ confFiles.length }}</el-tag>
            </div>
            <el-scrollbar class="list-scroll">
              <div
                v-for="f in confFiles"
                :key="f.path"
                class="file-item"
                :class="{ active: activeFile?.path === f.path }"
                @click="openFile(f)"
              >
                <el-icon><Document /></el-icon>
                <span class="file-name mono">{{
                  f.is_main ? t('servicesNginx.mainConfName') : f.rel
                }}</span>
              </div>
            </el-scrollbar>
          </div>
          <div class="editor-main">
            <div class="editor-bar">
              <div class="bar-left">
                <span class="mono path">{{
                  activeFile?.path || t('servicesNginx.selectFile')
                }}</span>
                <span v-if="activeFile" class="meta">{{ formatBytes(activeFile.size) }}</span>
                <el-tag v-if="activeFile?.is_main" size="small" type="warning">{{
                  t('servicesNginx.mainConfTag')
                }}</el-tag>
              </div>
              <div class="bar-right">
                <el-button
                  size="small"
                  :disabled="!dirty || !activeFile"
                  :loading="savingFile"
                  type="primary"
                  @click="saveFile"
                  >{{ t('servicesNginx.save') }}</el-button
                >
                <el-button
                  size="small"
                  :disabled="!dirty || !activeFile"
                  @click="reloadActiveFile"
                  >{{ t('servicesNginx.discardChanges') }}</el-button
                >
              </div>
            </div>
            <div class="editor-host">
              <CodeEditor v-model="fileContent" :path="activeFile?.path || ''" />
            </div>
            <div class="editor-tip">
              {{ t('servicesNginx.editorTip1') }}<code>nginx -t</code
              >{{ t('servicesNginx.editorTip2') }} <code>include …/sites-enabled/*.conf</code
              >{{ t('servicesNginx.editorTip3') }}
            </div>
          </div>
        </div>
      </div>

      <!-- WAF（ModSecurity）：可选能力，未安装时只给安装引导 -->
      <div v-show="page === 'waf'" class="nginx-config mt-3" v-loading="wafLoading">
        <!-- 未安装：说明缺什么，能装才给按钮 -->
        <el-result
          v-if="waf && !waf.installed"
          icon="warning"
          :title="t('waf.notInstalledTitle')"
          :sub-title="waf.hint"
        >
          <template #extra>
            <div class="waf-blockers">
              <div v-for="b in waf.blockers" :key="b" class="waf-blocker">· {{ b }}</div>
            </div>
            <el-button
              v-if="waf.enable_ready"
              type="success"
              :loading="wafEnabling"
              @click="doEnableWaf"
            >
              {{ t('waf.enable') }}
            </el-button>
            <el-button
              v-else
              type="primary"
              :disabled="!waf.installable"
              :loading="wafInstalling"
              @click="doInstallWaf"
            >
              {{ waf.installable ? t('waf.install') : t('waf.cannotInstall') }}
            </el-button>
            <div v-if="waf.enable_ready" class="waf-blocker">
              {{ t('waf.enableHint') }}
            </div>
          </template>
        </el-result>

        <template v-if="waf?.installed">
          <el-descriptions :column="2" border size="small" class="waf-desc">
            <el-descriptions-item :label="t('waf.engine')">
              <div class="waf-engine">
                <el-tag size="small" :type="waf.engine === 'On' ? 'success' : 'warning'">
                  {{ waf.engine || '-' }}
                </el-tag>
                <el-switch
                  size="small"
                  :model-value="waf.engine === 'On'"
                  :loading="wafEngineSwitching"
                  :active-text="t('waf.engineOn')"
                  :inactive-text="t('waf.engineDetection')"
                  @change="onWafEngineSwitch"
                />
              </div>
              <div class="waf-inline-tip">{{ t('waf.engineTip') }}</div>
            </el-descriptions-item>
            <el-descriptions-item :label="t('waf.crs')">
              <el-tag size="small" :type="waf.crs ? 'success' : 'info'">
                {{ waf.crs ? t('waf.deployed') : t('waf.missing') }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item :label="t('waf.module')">
              <span class="mono">{{ waf.module || '-' }}</span>
            </el-descriptions-item>
            <el-descriptions-item :label="t('waf.rulesDir')">
              <span class="mono">{{ waf.rules_dir }}</span>
            </el-descriptions-item>
            <el-descriptions-item :label="t('waf.auditLog')">
              <span class="mono">{{ waf.audit_log || '-' }}</span>
              <el-button
                size="small"
                class="waf-inline-btn"
                :disabled="!waf.audit_log"
                @click="openAudit"
              >
                {{ t('waf.viewAudit') }}
              </el-button>
            </el-descriptions-item>
          </el-descriptions>

          <el-alert
            type="warning"
            :closable="false"
            show-icon
            class="mt-3"
            :title="t('waf.saveTip')"
          />

          <el-table :data="waf.files" size="small" stripe class="mt-3">
            <el-table-column prop="rel" :label="t('waf.ruleFile')" min-width="240">
              <template #default="{ row }">
                <span class="mono">{{ row.rel }}</span>
              </template>
            </el-table-column>
            <el-table-column :label="t('waf.colOp')" width="120" align="right">
              <template #default="{ row }">
                <el-button size="small" @click="openRule(row.path)">
                  {{ t('waf.edit') }}
                </el-button>
              </template>
            </el-table-column>
          </el-table>
        </template>
      </div>
    </template>

    <!-- WAF：规则编辑 / 审计日志 / 安装日志（未安装时不会走到这里） -->
    <el-dialog v-model="ruleVisible" :title="rulePath" width="70%" top="6vh" append-to-body>
      <div v-loading="ruleLoading" class="waf-rule-editor">
        <CodeEditor v-model="ruleContent" :path="rulePath" />
      </div>
      <div class="waf-rule-tip">{{ t('waf.ruleTip') }}</div>
      <template #footer>
        <el-button @click="ruleVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="ruleSaving" @click="saveRule">
          {{ t('waf.save') }}
        </el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="auditVisible"
      :title="t('waf.auditTitle')"
      width="70%"
      top="6vh"
      append-to-body
    >
      <div class="waf-audit-bar">
        <el-input
          v-model="auditUid"
          :placeholder="t('waf.auditUidPlaceholder')"
          clearable
          style="max-width: 380px"
          @keyup.enter="runAuditQuery"
        />
        <el-button type="primary" :loading="auditLoading" @click="runAuditQuery">
          {{ t('waf.auditQuery') }}
        </el-button>
        <el-button @click="openAudit">{{ t('waf.auditTail') }}</el-button>
      </div>

      <!-- 按 unique_id 查到的结构化明细 -->
      <div v-if="auditDetail && auditDetail.found" class="waf-audit-detail">
        <el-descriptions :column="2" border size="small">
          <el-descriptions-item :label="t('waf.auditReqLine')">
            {{ auditDetail.request?.line }}
          </el-descriptions-item>
          <el-descriptions-item :label="t('waf.auditClient')">
            {{ auditDetail.header?.client_ip }}:{{ auditDetail.header?.client_port }}
          </el-descriptions-item>
          <el-descriptions-item :label="t('waf.auditTime')">
            {{ auditDetail.header?.timestamp }}
          </el-descriptions-item>
          <el-descriptions-item :label="t('waf.auditServer')">
            {{ auditDetail.header?.server_ip }}:{{ auditDetail.header?.server_port }}
          </el-descriptions-item>
        </el-descriptions>

        <div class="waf-audit-sub">{{ t('waf.auditRules') }}</div>
        <el-table :data="auditDetail.messages" size="small" border>
          <el-table-column prop="id" label="ID" width="110" />
          <el-table-column prop="action" :label="t('waf.auditAction')" width="200" />
          <el-table-column prop="severity" :label="t('waf.auditSeverity')" width="120" />
          <el-table-column prop="msg" :label="t('waf.auditMsg')" min-width="200" />
          <el-table-column :label="t('waf.auditRule')" min-width="220">
            <template #default="{ row }">
              {{ row.file.split('/').pop() }} : {{ row.line }}
            </template>
          </el-table-column>
          <el-table-column prop="data" :label="t('waf.auditData')" min-width="220" />
        </el-table>

        <el-collapse class="waf-audit-raw">
          <el-collapse-item :title="t('waf.auditRaw')">
            <pre class="mono">{{ auditDetail.raw }}</pre>
          </el-collapse-item>
        </el-collapse>
      </div>

      <el-alert
        v-else-if="auditDetail && !auditDetail.found"
        type="info"
        :closable="false"
        :title="t('waf.auditNotFound')"
      />

      <!-- 尾部模式（未输入 unique_id） -->
      <pre v-else class="waf-audit mono">{{ auditText || t('waf.auditEmpty') }}</pre>
    </el-dialog>

    <AppStoreLogDrawer ref="logDrawer" />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Document } from '@/icons'
import CodeEditor from '@/components/CodeEditor.vue'
import NginxGlobal from './nginx/global.vue'
import {
  controlNginx,
  getNginxStatus,
  listNginxConfs,
  readNginxConf,
  saveNginxConf,
  setNginxDefaultVhost,
  type NginxConfFile,
  type NginxStatus,
} from '@/api/serverNginx.ts'
import StreamConf from './stream.vue'
import AppStoreLogDrawer from '@/components/AppStoreLogDrawer.vue'
import { getTask } from '@/api/task.ts'
import {
  getWafAudit,
  getWafConfRead,
  getWafStatus,
  setWafEngine,
  enableWaf,
  type WafAuditData,
  installWaf,
  saveWafConf,
  type WafStatus,
} from '@/api/waf.ts'

const { t } = useI18n()
const router = useRouter()
const route = useRoute()
const bootLoading = ref(true)
const busy = ref(false)
const acting = ref('')
const installed = ref<boolean | null>(null)
const status = ref<NginxStatus>({})
/** conf = 主配置可视化；stream = 四层转发规则；files = 文件编辑；waf = WAF */
const PAGE_KEYS = ['conf', 'stream', 'files', 'waf'] as const
type NginxPage = (typeof PAGE_KEYS)[number]
/** 初始子页：优先 ?page=（配合 ?tab=nginx，刷新后停留在原子页） */
const page = ref<NginxPage>(
  (PAGE_KEYS as readonly string[]).includes(String(route.query.page))
    ? (String(route.query.page) as NginxPage)
    : 'conf',
)

function setPage(p: NginxPage) {
  page.value = p
  router.replace({ query: { ...route.query, tab: 'nginx', page: p } })
}

const running = computed(() => !!status.value.running)
const versionText = computed(() => {
  const m = (status.value.version || '').match(/nginx\/([\d.]+)/)
  return m ? m[1] : status.value.version || '-'
})

/* ---------- 默认站点（IP 访问） ---------- */
const savingDefaultVhost = ref(false)
const ipAccess = computed<boolean>({
  get: () => status.value.default_ip_access ?? false,
  set: (v: boolean) => {
    status.value.default_ip_access = v
  },
})

async function saveDefaultVhost(v: boolean | string | number) {
  const enable = !!v
  savingDefaultVhost.value = true
  try {
    const res = await setNginxDefaultVhost(enable)
    const stateText = t(enable ? 'servicesNginx.defaultEnabled' : 'servicesNginx.defaultDisabled')
    const tail = res.data?.reason
      ? t('servicesNginx.defaultReason', { reason: res.data.reason })
      : t('servicesNginx.defaultReloaded')
    ElMessage.success(stateText + tail)
    await refreshStatus()
  } catch {
    /* interceptor：设置失败时刷新状态回退开关 */
    await refreshStatus()
  } finally {
    savingDefaultVhost.value = false
  }
}

/* ---------- 文件编辑 ---------- */
const confFiles = ref<NginxConfFile[]>([])
const activeFile = ref<NginxConfFile | null>(null)
const fileContent = ref('')
let fileOriginal = ''
const savingFile = ref(false)

const dirty = computed(() => fileContent.value !== fileOriginal)

async function refreshConfList() {
  try {
    const res = await listNginxConfs()
    if (res.code !== 0) return
    const data = res.data
    confFiles.value = data.files
    if (!activeFile.value && confFiles.value.length) {
      await openFile(confFiles.value.find((f) => f.is_main) || confFiles.value[0])
    } else if (activeFile.value) {
      const cur = confFiles.value.find((f) => f.path === activeFile.value?.path)
      if (!cur) activeFile.value = null
    }
  } catch {
    /* interceptor */
  }
}

async function openFile(f: NginxConfFile) {
  if (activeFile.value && dirty.value) {
    try {
      await ElMessageBox.confirm(t('servicesNginx.unsavedSwitch'), t('common.tip'), {
        type: 'warning',
      })
    } catch {
      return
    }
  }
  activeFile.value = f
  try {
    const res = await readNginxConf(f.path)
    if (res.code !== 0) return
    fileContent.value = res.data.content
    fileOriginal = res.data.content
  } catch {
    /* interceptor */
  }
}

function reloadActiveFile() {
  if (activeFile.value) {
    fileContent.value = fileOriginal
    ElMessage.info(t('servicesNginx.discarded'))
  }
}

async function saveFile() {
  if (!activeFile.value || !dirty.value) return
  savingFile.value = true
  try {
    const res = await saveNginxConf(activeFile.value.path, fileContent.value)
    fileOriginal = fileContent.value
    ElMessage.success(
      res.data?.reloaded
        ? t('servicesNginx.fileSavedReloaded')
        : t('servicesNginx.fileSavedReason', {
            reason: res.data?.reason || t('servicesNginx.fileSavedPending'),
          }),
    )
  } catch {
    /* interceptor */
  } finally {
    savingFile.value = false
  }
}

/* ---------- 状态与控制 ---------- */
async function refreshStatus() {
  busy.value = true
  try {
    const res = await getNginxStatus()
    if (res.code === 0) {
      status.value = res.data
      installed.value = res.data.installed ?? false
    }
  } catch {
    /* interceptor */
  } finally {
    busy.value = false
  }
}

async function refreshAll() {
  bootLoading.value = true
  await refreshStatus()
  if (installed.value) {
    await refreshConfList()
  }
  bootLoading.value = false
}

const ctrlLabels = computed<Record<string, string>>(() => ({
  reload: t('servicesNginx.reload'),
  restart: t('servicesNginx.restart'),
  start: t('servicesNginx.start'),
  stop: t('servicesNginx.stop'),
}))

async function doControl(action: 'reload' | 'restart' | 'start' | 'stop') {
  const warn = action === 'stop' || action === 'restart'
  try {
    await ElMessageBox.confirm(
      t('servicesNginx.controlConfirm', { action: ctrlLabels.value[action] }),
      t('common.tip'),
      {
        type: warn ? 'warning' : 'info',
      },
    )
  } catch {
    return
  }
  acting.value = action
  try {
    const res = await controlNginx(action)
    ElMessage.success(
      res.message ?? t('servicesNginx.actionOk', { action: ctrlLabels.value[action] }),
    )
    await refreshStatus()
  } catch {
    /* interceptor */
  } finally {
    acting.value = ''
  }
}

/* ---------- WAF（ModSecurity）：可选能力，未安装即只显示安装引导 ---------- */
const waf = ref<WafStatus | null>(null)
const wafLoading = ref(false)
const wafInstalling = ref(false)
const wafEngineSwitching = ref(false)
const wafEnabling = ref(false)
const logDrawer = ref<InstanceType<typeof AppStoreLogDrawer> | null>(null)
const ruleVisible = ref(false)
const rulePath = ref('')
const ruleContent = ref('')
const ruleLoading = ref(false)
const ruleSaving = ref(false)
const auditVisible = ref(false)
const auditText = ref('')
const auditUid = ref('')
const auditLoading = ref(false)
const auditDetail = ref<WafAuditData | null>(null)
/** 安装任务轮询（编译分钟级），完成后刷新状态 */
let wafTimer: number | undefined

async function loadWaf() {
  wafLoading.value = true
  try {
    const res = await getWafStatus()
    waf.value = res.data
  } catch {
    /* interceptor 已提示 */
  } finally {
    wafLoading.value = false
  }
}

/** 盯安装任务到终态：编译期间日志在抽屉里流式输出 */
function watchWafTask(runId: string) {
  logDrawer.value?.openDrawer(runId, t('waf.installTitle'))
  if (wafTimer) window.clearInterval(wafTimer)
  wafTimer = window.setInterval(async () => {
    try {
      const res = await getTask(runId)
      const st = res.data?.status || res.data?.task?.status
      if (st === 'success' || st === 'failed' || st === 'canceled') {
        window.clearInterval(wafTimer)
        wafTimer = undefined
        await loadWaf()
      }
    } catch {
      if (wafTimer) window.clearInterval(wafTimer)
      wafTimer = undefined
    }
  }, 3000)
}

/** 一键开启：组件已在盘上、只差挂到 nginx（补 load_module + 启用文件 + 重载） */
async function doEnableWaf() {
  wafEnabling.value = true
  try {
    const res = await enableWaf()
    ElMessage.success(res.data?.reload || t('waf.enableDone'))
    await loadWaf()
  } catch {
    /* interceptor 已提示 */
  } finally {
    wafEnabling.value = false
  }
}

/** 拦截开关：On = 拦截，DetectionOnly = 只记录（装完默认） */
async function onWafEngineSwitch(val: boolean) {
  wafEngineSwitching.value = true
  try {
    const res = await setWafEngine(val ? 'On' : 'DetectionOnly')
    ElMessage.success(res.data?.reload || t('waf.engineSaved'))
    await loadWaf()
  } catch {
    /* interceptor 已提示 */
  } finally {
    wafEngineSwitching.value = false
  }
}

async function doInstallWaf() {
  wafInstalling.value = true
  try {
    const res = await installWaf()
    if (res.data?.task_id) {
      watchWafTask(res.data.task_id)
    } else {
      await loadWaf()
    }
  } catch {
    /* interceptor 已提示 */
  } finally {
    wafInstalling.value = false
  }
}

async function openRule(path: string) {
  rulePath.value = path
  ruleVisible.value = true
  ruleLoading.value = true
  try {
    const res = await getWafConfRead(path)
    ruleContent.value = res.data.content
  } catch {
    ruleVisible.value = false
  } finally {
    ruleLoading.value = false
  }
}

async function saveRule() {
  ruleSaving.value = true
  try {
    const res = await saveWafConf(rulePath.value, ruleContent.value)
    ElMessage.success(res.data?.reload || res.message || t('servicesCommon.opSuccess'))
    ruleVisible.value = false
  } catch {
    /* 规则不合法时后端已回滚并给出 nginx -t 的报错 */
  } finally {
    ruleSaving.value = false
  }
}

async function openAudit() {
  auditVisible.value = true
  auditUid.value = ''
  auditDetail.value = null
  auditText.value = ''
  try {
    const res = await getWafAudit(200)
    auditText.value = res.data.content
  } catch {
    /* interceptor 已提示 */
  }
}

/** 按 unique_id 反查单条审计明细（来自 error_log 末尾的 [unique_id "..."]） */
async function runAuditQuery() {
  const uid = auditUid.value.trim()
  if (!uid) {
    openAudit()
    return
  }
  auditLoading.value = true
  auditDetail.value = null
  try {
    const res = await getWafAudit(200, uid)
    auditDetail.value = res.data
  } catch {
    /* interceptor 已提示 */
  } finally {
    auditLoading.value = false
  }
}

function formatBytes(n: number): string {
  if (n >= 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`
  if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${n} B`
}

onMounted(async () => {
  await refreshAll()
  // WAF 与 nginx 之间是"可选"关系：nginx 装了也未必有 WAF，状态各自独立
  loadWaf()
})
onUnmounted(() => {
  if (wafTimer) window.clearInterval(wafTimer)
})
</script>

<style scoped>
.nginx-page {
  min-height: 60vh;
}
.nginx-pills {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px;
  margin: 12px 0;
  background: var(--el-fill-color-light);
  border-radius: 10px;
}
.nginx-pill {
  padding: 6px 14px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--el-text-color-regular);
  font-size: 13px;
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
}
.nginx-pill:hover {
  color: var(--el-color-primary);
}
.nginx-pill.is-active {
  background: var(--el-color-primary);
  color: #fff;
  font-weight: 500;
}
.nginx-config {
  min-height: 60vh;
}
.mt-3 {
  margin-top: 12px;
}
.mono {
  font-family: 'JetBrains Mono', Menlo, Consolas, monospace;
}
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.tip {
  margin-top: 6px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.top-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-wrap: wrap;
  gap: 10px;
}
.title {
  display: flex;
  align-items: center;
  gap: 8px;
}
.t-name {
  font-weight: 600;
  font-size: 15px;
}
.actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.path {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  max-width: 420px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 文件编辑 */
.editor-layout {
  display: flex;
  gap: 12px;
  min-height: 560px;
}
.file-list {
  width: 280px;
  flex-shrink: 0;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 6px;
  display: flex;
  flex-direction: column;
}
.list-head {
  padding: 8px 10px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  font-weight: 600;
}
.list-scroll {
  flex: 1;
}
.file-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 10px;
  cursor: pointer;
  font-size: 13px;
  color: var(--el-text-color-regular);
  transition: background 0.15s;
}
.file-item:hover {
  background: var(--el-fill-color-light);
}
.file-item.active {
  background: var(--el-color-primary-light-9);
  color: var(--el-color-primary);
}
.file-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
}
.editor-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 6px;
  min-width: 0;
}
.editor-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--el-border-color-lighter);
  flex-wrap: wrap;
}
.bar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.bar-left .path {
  max-width: 560px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  color: var(--el-text-color-primary);
}
.meta {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.editor-host {
  flex: 1;
  min-height: 460px;
}
.editor-tip {
  padding: 6px 10px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  border-top: 1px solid var(--el-border-color-lighter);
  line-height: 1.6;
}

/* WAF */
.waf-blockers {
  max-width: 620px;
  margin: 0 auto 12px;
  text-align: left;
  font-size: 12px;
  color: var(--el-text-color-regular);
  line-height: 1.9;
}
.waf-blocker {
  padding-left: 4px;
}
.waf-desc {
  margin-bottom: 4px;
}
.waf-engine {
  display: flex;
  align-items: center;
  gap: 8px;
}
.waf-inline-tip {
  margin-left: 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.waf-inline-btn {
  margin-left: 8px;
}
.waf-rule-editor {
  height: 52vh;
}
.waf-rule-tip {
  margin-top: 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  line-height: 1.7;
}
.waf-audit {
  max-height: 60vh;
  overflow: auto;
  margin: 0;
  padding: 10px;
  font-size: 12px;
  line-height: 1.6;
  background: var(--el-fill-color-light);
  border-radius: 4px;
  white-space: pre-wrap;
  word-break: break-all;
}
.waf-audit-bar {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 12px;
}
.waf-audit-detail {
  margin-top: 4px;
}
.waf-audit-sub {
  font-weight: 600;
  margin: 16px 0 8px;
  color: var(--el-text-color-primary);
}
.waf-audit-raw {
  margin-top: 12px;
}
.waf-audit-raw .mono {
  max-height: 40vh;
  overflow: auto;
  font-size: 12px;
  line-height: 1.6;
  background: var(--el-fill-color-light);
  border-radius: 4px;
  padding: 10px;
  white-space: pre-wrap;
  word-break: break-all;
}

/* 默认站点（IP 访问） */
.dvh-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}
.dvh-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 6px;
}
.dvh-desc {
  font-size: 12px;
  color: var(--el-text-color-secondary);
  line-height: 1.7;
  max-width: 720px;
}
.dvh-path {
  margin-top: 6px;
  font-size: 12px;
  color: var(--el-color-primary);
  word-break: break-all;
}
</style>
