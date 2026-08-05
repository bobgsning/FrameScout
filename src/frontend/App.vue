<!--
  SPDX-License-Identifier: Apache-2.0
  FrameScout — Vue 3 Frontend
  =============================
  Main UI for FrameScout desktop application.

  Features:
  - Search console (text + image-to-image)
  - Real-time scan progress with batch updates
  - Visual clustering view with adjustable threshold
  - Smart folders (save/apply/delete)
  - Pagination and result grid
  - Pro/Trial license badge with activation modal
  - Video frame preview with timeout handling

  Architecture: Composition API + TypeScript
  Communication: Tauri IPC commands + event listeners
-->

<!-- Copyright (c) 2026 AetherFlow Labs Inc. -->

<!-- The "framescout-logo" symbol is a trademark of AetherFlow Labs Inc. See LOGO_LICENSE.md for terms. -->

<template>
  <!-- ============================================================
       Global SVG Asset Library (invisible)
       - Define all gradients (28 total) once
       - Define Logo paths as a <symbol>, shared by splash & header
       - Always present in DOM, unaffected by v-if toggles
       ============================================================ -->
  <svg width="0" height="0" style="position:absolute; width:0; height:0; overflow:hidden;" aria-hidden="true">
    <defs>
      <!-- ===== Base Gradients (14 raw color definitions) ===== -->
      <linearGradient id="grad24">
        <stop offset="0%" stop-color="#821fc6" stop-opacity="1"/>
        <stop offset="100%" stop-color="#2493db" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad30">
        <stop offset="0%" stop-color="#8333ff" stop-opacity="1"/>
        <stop offset="100%" stop-color="#67e5e5" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad35">
        <stop offset="0%" stop-color="#e5a800" stop-opacity="1"/>
        <stop offset="100%" stop-color="#ebff1a" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad37">
        <stop offset="0%" stop-color="#227ec3" stop-opacity="1"/>
        <stop offset="100%" stop-color="#2fe9e9" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad39">
        <stop offset="0%" stop-color="#ffee82" stop-opacity="1"/>
        <stop offset="100%" stop-color="#ff8000" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad41">
        <stop offset="0%" stop-color="#f9f922" stop-opacity="1"/>
        <stop offset="100%" stop-color="#ff6619" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad44">
        <stop offset="0%" stop-color="#bf99ff" stop-opacity="1"/>
        <stop offset="100%" stop-color="#abeeee" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad46">
        <stop offset="0%" stop-color="#d3a6f2" stop-opacity="1"/>
        <stop offset="100%" stop-color="#d5e8f6" stop-opacity="0.8"/>
      </linearGradient>
      <linearGradient id="grad48">
        <stop offset="0%" stop-color="#a9d1ef" stop-opacity="0.8"/>
        <stop offset="100%" stop-color="#baf8f8" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad51">
        <stop offset="0%" stop-color="#fee59b" stop-opacity="0.8"/>
        <stop offset="100%" stop-color="#f8ffb3" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad54">
        <stop offset="0%" stop-color="#fff5b3" stop-opacity="0.8"/>
        <stop offset="100%" stop-color="#ffcc99" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad56">
        <stop offset="0%" stop-color="#ffff80" stop-opacity="1"/>
        <stop offset="100%" stop-color="#ffcdcd" stop-opacity="0.8"/>
      </linearGradient>
      <linearGradient id="grad58">
        <stop offset="0%" stop-color="#e7eec3" stop-opacity="1"/>
        <stop offset="100%" stop-color="#c3e6ee" stop-opacity="1"/>
      </linearGradient>
      <linearGradient id="grad61">
        <stop offset="0%" stop-color="#d4f0f7" stop-opacity="0.5"/>
        <stop offset="100%" stop-color="#f8f0d3" stop-opacity="0.5"/>
      </linearGradient>

      <!-- ===== Instantiated Gradients (14 coordinate-aware copies) ===== -->
      <linearGradient id="grad25" href="#grad24" xlink:href="#grad24" x1="184" y1="346" x2="482.02686" y2="348" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad32" href="#grad30" xlink:href="#grad30" x1="272" y1="200" x2="778" y2="200" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad36" href="#grad35" xlink:href="#grad35" x1="152.3334" y1="796.60059" x2="479.6666" y2="567.39941" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad38" href="#grad37" xlink:href="#grad37" x1="492" y1="348" x2="838" y2="348" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad40" href="#grad39" xlink:href="#grad39" x1="515.21161" y1="565.11707" x2="814.78839" y2="774.88293" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad42" href="#grad41" xlink:href="#grad41" x1="489.6843" y1="738.24103" x2="500.3157" y2="859.75873" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad45" href="#grad44" xlink:href="#grad44" x1="272" y1="200" x2="778" y2="200" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad47" href="#grad46" xlink:href="#grad46" x1="183.99664" y1="347" x2="482.03021" y2="347" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad49" href="#grad48" xlink:href="#grad48" x1="492" y1="348" x2="838" y2="348" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad53" href="#grad51" xlink:href="#grad51" x1="152.57378" y1="796.94312" x2="479.42621" y2="567.05688" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad55" href="#grad54" xlink:href="#grad54" x1="516" y1="564" x2="814" y2="776" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad57" href="#grad56" xlink:href="#grad56" x1="489.81357" y1="738.22986" x2="500.18643" y2="859.7699" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad59" href="#grad58" xlink:href="#grad58" x1="501" y1="479.64279" x2="501" y2="522.35724" gradientUnits="userSpaceOnUse" />
      <linearGradient id="grad62" href="#grad61" xlink:href="#grad61" x1="501" y1="479.64279" x2="501" y2="522.35724" gradientUnits="userSpaceOnUse" />

      <!-- ===== Logo Symbol Definition (7 paths merged into one unit) ===== -->
      <symbol id="framescout-logo" viewBox="0 0 1000 1000">
        <g>
          <path style="fill:url(#grad25);fill-rule:nonzero;stroke:url(#grad47);stroke-width:5;stroke-dasharray:none;stroke-linejoin:round;stroke-linecap:round" d="M 360,500 150,200 500,260 v 190 z" />
          <path style="fill:url(#grad32);fill-opacity:1;fill-rule:nonzero;stroke:url(#grad45);stroke-opacity:1;stroke-width:5;stroke-dasharray:none;stroke-linejoin:round;paint-order:normal;stroke-linecap:round" d="M 500,260 150,200 500,140 850,200 Z" />
          <path style="fill:url(#grad36);fill-rule:nonzero;stroke:url(#grad53);stroke-width:5;stroke-dasharray:none;stroke-linejoin:round;stroke-linecap:round" d="M 360,500 150,800 500,720 V 550 Z" />
          <path style="fill:url(#grad38);fill-rule:nonzero;stroke:url(#grad49);stroke-width:5;stroke-dasharray:none;stroke-linejoin:round;stroke-linecap:round" d="M 640,500 500,450 V 260 l 350,-60 z" />
          <path style="fill:url(#grad40);fill-rule:nonzero;stroke:url(#grad55);stroke-width:5;stroke-dasharray:none;stroke-linejoin:round;stroke-linecap:round" d="M 500,550 640,500 850,800 500,720 Z" />
          <path style="fill:url(#grad42);fill-rule:nonzero;stroke:url(#grad57);stroke-width:5;stroke-dasharray:none;stroke-linejoin:round;stroke-linecap:round" d="m 500,720 -350,80 350,80 350,-80 z" />
          <path style="fill:url(#grad59);fill-opacity:1;fill-rule:nonzero;stroke:url(#grad62);stroke-width:5;stroke-linecap:round;stroke-linejoin:round;stroke-dasharray:none;stroke-opacity:1;paint-order:normal" d="m 500,450 -140,50 140,50 140,-50 z" />
        </g>
      </symbol>
    </defs>
  </svg>

  <!-- ============================================================
       Splash Screen
       ============================================================ -->
  <div v-if="!engineReady" class="neural-splash">
    <div class="logo-wrapper" style="margin-bottom: 35px;">
      <!-- Single line, referencing the global symbol -->
      <svg width="80" height="80" viewBox="0 0 1000 1000" fill="none" class="brand-svg-glow">
        <use href="#framescout-logo"/>
      </svg>
    </div>
    <div class="loader-ring"></div>
    <h2 class="splash-title">FRAME SCOUT NEURAL LINK ESTABLISHING...</h2>
    <p class="splash-subtitle">Loading large CLIP & EasyOCR models into high-dimensional memory, please wait...</p>
  </div>

  <div v-else class="app-container">
    <!-- ============================================================
         Header (title bar)
         ============================================================ -->
    <div class="brand-header-box">
      <!-- Same single line, just different size -->
      <svg width="36" height="36" viewBox="0 0 1000 1000" fill="none">
        <use href="#framescout-logo"/>
      </svg>
      <h1 class="brand-header">FrameScout: Global Edition</h1>

      <!-- License status button (right-aligned) -->
      <div class="license-badge" @click="showActivateModal = true">
        <span v-if="licenseStatus.is_pro" class="badge-pro">💎 Pro Edition</span>
        <span v-else class="badge-trial">⚡ Trial Mode</span>
      </div>
    </div>

    <!-- Activation Modal -->
    <div v-if="showActivateModal" class="modal-overlay" @click.self="showActivateModal = false">
      <div
        class="modal-card"
        :class="licenseStatus.is_pro ? 'pro-theme' : 'trial-theme'"
        role="dialog"
        aria-modal="true"
      >
        <h2>{{ licenseStatus.is_pro ? '💎' : '⚡' }} {{ licenseStatus.is_pro ? 'Your License - Pro' : 'Activate FrameScout Pro' }}</h2>
        <p class="modal-sub">Enter your email and the License Key provided after purchase.</p>

        <div class="form-group">
          <label>Email Address</label>
          <input v-model="licenseEmail" type="email" placeholder="your@email.com" class="custom-input modal-input" />
        </div>

        <div class="form-group">
          <label>License Key</label>
          <textarea v-model="licenseKeyInput" placeholder="Paste your Base64 License Key here..." class="custom-textarea modal-textarea"></textarea>
        </div>

        <p v-if="activateMsg" :class="['activate-msg', activateSuccess ? 'msg-success' : 'msg-error']">{{ activateMsg }}</p>

        <div class="modal-actions">
          <button @click="showActivateModal = false" class="btn btn-secondary">Cancel</button>
          <button @click="submitActivation" class="btn btn-primary">Activate License</button>
        </div>
      </div>
    </div>

    <!-- Top Action Bar -->
    <div class="top-action-bar">
      <input v-model="folderPath" id="folderPath" @input="savePath" type="text" placeholder="Paste folder path here, or click Browse..." class="custom-input path-input" />

      <button @click="selectFolder" class="btn btn-secondary">📂 Browse</button>
      <select v-model="scanMode" id="scanMode" class="custom-select">
        <option value="all">All Media</option>
        <option value="image">Image Only</option>
        <option value="video">Video Only</option>
      </select>

      <button @click="startScan" :disabled="isScanning || !folderPath" class="btn btn-primary">
        {{ isScanning ? 'Indexing Data...' : 'Start Indexing' }}
      </button>

      <button @click="toggleClustering" class="btn btn-cluster" title="Group visually similar images">
        {{ isClusteringView ? '🖼️ Standard View' : '🧩 Visual Clusters' }}
      </button>

      <button @click="cleanGhosts" class="btn btn-danger" title="Remove physically deleted records">🧹 Purge Ghosts</button>
    </div>

    <div v-if="smartFolders.length > 0" class="smart-folders-bar">
      <span class="smart-folder-label">⭐ Smart Folders:</span>
      <div v-for="sf in smartFolders" :key="sf.id" class="smart-folder-pill" @click="applySmartFolder(sf)">
        <span class="sf-name">📁 {{ sf.name }}</span>
        <span class="sf-delete" @click.stop="removeSmartFolder(sf.id)">✕</span>
      </div>
    </div>

    <!-- Live Status Monitor -->
    <div class="monitor-wrapper">
      <p v-if="scanMsg" class="scan-msg">{{ scanMsg }}</p>

      <div v-if="isScanning || currentFile" class="bus-panel">
        <div class="bus-header">
          <span class="bus-title">⚡ Real-time Extraction Bus</span>
          <span class="bus-status" :class="{ 'status-warning': currentStatus.includes('Batch') }">{{ currentStatus }}</span>
        </div>

        <div v-if="scanProgress.total > 0" class="progress-box">
          <div class="progress-info">
            <span>Parsing Progress: {{ scanProgress.current }} / {{ scanProgress.total }}, </span>
            <span class="progress-percentage">{{ ((scanProgress.current / scanProgress.total) * 100).toFixed(1) }}%</span>
          </div>
          <div class="progress-track">
            <div class="progress-fill" :style="{ width: ((scanProgress.current / scanProgress.total) * 100) + '%' }"></div>
          </div>
        </div>

        <div class="current-file-text">> {{ currentFile || 'Awaiting signal...' }}</div>
      </div>
    </div>

    <!-- Floating new-file banner (placed after monitor panel, before clustering/search) -->
    <!-- Floating new-file banner -->
    <div v-if="showIncomingBanner && incomingCount > 0"
      class="incoming-banner"
      @click="acceptIncomingFiles">
      <span class="banner-text">📥 {{ incomingCount }} new {{ incomingCount === 1 ? 'image has' : 'images have' }} been indexed. Click to view.</span>
      <span class="banner-close" @click.stop="dismissIncomingBanner">✕</span>
    </div>

    <!-- Visual Clustering View -->
    <div v-if="isClusteringView" class="clustering-container">
      <div class="clustering-header">
        <h2>🧩 Visual Similarity Clusters</h2>
        <div class="cluster-controls">
          <label>Similarity Threshold: </label>
          <input type="range" v-model.number="clusterThreshold" min="0.65" max="0.95" step="0.05" @change="fetchClusters" />
          <span>{{ (clusterThreshold * 100).toFixed(0) }}%</span>
          <button @click="fetchClusters" class="btn btn-secondary btn-sm">Re-cluster</button>
        </div>
      </div>

      <div v-if="clusters.length === 0" class="empty-clusters">
        <p>No visually similar groups found at current threshold.</p>
        <p class="empty-hint">Try lowering the similarity threshold to discover more groups.</p>
      </div>

      <div class="cluster-grid">
        <div v-for="group in clusters" :key="group.group_id" class="cluster-card">
          <div class="cluster-badge">Group #{{ group.group_id }} ({{ group.member_paths.length }} items)</div>
          <div class="cluster-thumbnails">
            <img v-for="(path, idx) in group.member_paths" :key="idx" :src="getAssetUrl(path)" class="cluster-thumb" />
          </div>
        </div>
      </div>
    </div>

    <!-- Standard Search Console -->
    <div v-else>
      <div class="search-console">
        <div class="search-options">
          <label><input type="checkbox" v-model="s_filename" @change="resetAndSearch"/> 📁 Filename</label>
          <label><input type="checkbox" v-model="s_note" @change="resetAndSearch"/> 📝 Note</label>
          <label><input type="checkbox" v-model="s_ocr" @change="resetAndSearch"/> 👁️ OCR</label>
          <label><input type="checkbox" v-model="s_vector" @change="resetAndSearch"/> 🧠 Semantic AI</label>

          <div class="option-divider"></div>

          <label>
            Items/Page:
            <select v-model="pageSize" @change="resetAndSearch" class="custom-select-sm">
              <option :value="4">4</option><option :value="8">8</option><option :value="16">16</option><option :value="50">50</option>
            </select>
          </label>
        </div>

        <div v-if="isImageSearch && searchImagePath" class="visual-target-banner">
          <img :src="getAssetUrl(searchImagePath)" class="target-img" />
          <div class="target-info">
            <p class="target-title">🎯 Visual Target</p>
            <p class="target-path">{{ searchImagePath }}</p>
          </div>
          <button @click="clearImageSearch" class="btn btn-danger btn-sm">✖ Cancel</button>
        </div>

        <div class="search-bar">
          <input v-model="searchQuery" id="searchQuery" @keyup.enter="resetAndSearch" type="text" placeholder="Multi-dim search: description, filename, or notes..." class="custom-input search-input" />
          <button @click="resetAndSearch" class="btn btn-primary search-btn">Search</button>

          <button @click="searchByImageAction" class="btn btn-outline-primary" title="Select an image to find similar frames!">
            🖼️ Similar
          </button>

          <button v-if="searchQuery" @click="promptSaveSmartFolder" class="btn btn-secondary" title="Save this search as a Smart Folder">⭐ Save Smart</button>
        </div>

        <p v-if="searchMsg" class="error-msg">{{ searchMsg }}</p>
      </div>

      <!-- Results Area -->
      <div class="results-grid">
        <div v-for="(item, index) in results" :key="index" class="result-card">

          <div v-if="item.isMissing" class="missing-card">
            <span class="missing-icon">👻</span>
            <span>File Unavailable</span>
          </div>
          <template v-else>
            <img v-if="!isVideo(item.path)" :src="getAssetUrl(item.path)" @error="item.isMissing=true" class="media-preview" />
            <video v-else :src="getAssetUrl(item.path)+'#t='+item.timestamp" @error="handleVideoError(item)"
              @loadeddata="handleVideoLoaded(item)"
              controls preload="metadata"
              class="media-preview"
            ></video>
          </template>

          <div class="card-content">
            <p class="file-path" v-html="highlight(item.path)"></p>

            <div class="card-meta-row">
              <p v-if="isVideo(item.path)" class="badge-timestamp">⏱️ Sec {{ item.timestamp }}</p>
              <p v-else class="type-text">🖼️ Static Image</p>
              <p class="match-score">{{ formatScore(item.score, isImageSearch) }}</p>
            </div>

            <div class="tags-wrapper">
              <span v-for="tag in item.matched_tags.filter((t: string) => !t.includes('Semantic'))" :key="tag" class="tag-badge">
                {{ tag }}
              </span>

              <span v-if="getNumericScore(item.score, isImageSearch) >= 60.0 && item.matched_tags.length === 0" class="tag-badge semantic-tag">
                🧠 Semantic
              </span>

              <span v-if="getNumericScore(item.score, isImageSearch) < 60.0 && item.matched_tags.length === 0" class="tag-badge low-tag">
                👻 Low Confidence
              </span>
            </div>

            <div v-if="item.ocr_text" class="ocr-panel">
              <p class="ocr-label">👁️ Extracted Text:</p>
              <p class="ocr-content" :class="{ 'ocr-collapsed': !item.expandOcr }" v-html="highlight(item.ocr_text)"></p>
              <button v-if="item.ocr_text.length > 50" @click="item.expandOcr = !item.expandOcr" class="btn-text">
                {{ item.expandOcr ? 'Collapse 🔼' : 'Expand 🔽' }}
              </button>
            </div>

            <div class="note-panel">
              <p class="note-label">📝 Personal Note (Markdown):</p>
              <textarea v-model="item.user_note" id="userNote" @blur="saveNote(item)" placeholder="Enter notes..." class="custom-textarea"></textarea>
            </div>
          </div>
        </div>
      </div>

      <!-- Pagination -->
      <div v-if="totalResults > 0" class="pagination-wrapper">
        <button @click="changePage(-1)" :disabled="currentPage === 1" class="btn btn-secondary btn-sm">Prev</button>

        <div class="page-info">
          <span class="page-current">Page {{ currentPage }} / {{ totalPages }}</span>
          <p class="page-total">(Total {{ totalResults }} hits)</p>
        </div>

        <button @click="changePage(1)" :disabled="currentPage >= totalPages" class="btn btn-secondary btn-sm">Next</button>

        <div class="jump-box">
          <span>Jump to</span>
          <input v-model.number="jumpPage" id="jumpPage" @keyup.enter="doJump" type="number" min="1" :max="totalPages" class="jump-input" />
          <button @click="doJump" class="btn btn-primary btn-sm">GO</button>
        </div>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'

// Define interfaces here
interface SearchResult {
  path: string
  timestamp: number
  score: number
  matched_tags: string[]
  ocr_text: string
  user_note: string
  isMissing?: boolean   // optional property
  expandOcr?: boolean   // optional property
}

interface ClusterGroup {
  group_id: number
  member_paths: string[]
}

interface SmartFolder {
  id: number
  name: string
  query_text: string
  use_vector: boolean
  use_ocr: boolean
  use_note: boolean
  use_filename: boolean
}

const engineReady = ref(false)
const folderPath = ref('')
const scanMode = ref('all')
const searchQuery = ref('')
const isScanning = ref(false)
const scanMsg = ref('')
const searchMsg = ref('')

const s_filename = ref(true)
const s_note = ref(true)
const s_ocr = ref(true)
const s_vector = ref(true)

const isImageSearch = ref(false)
const searchImagePath = ref('')

const results = ref<SearchResult[]>([])
const currentStatus = ref('')
const currentFile = ref('')
const scanProgress = ref({ current: 0, total: 0 })

const currentPage = ref(1)
const pageSize = ref(8)
const jumpPage = ref(1)

const totalResults = ref(0)
const totalPages = computed(() => Math.max(1, Math.ceil(totalResults.value / pageSize.value)))

const isClusteringView = ref(false)
const clusterThreshold = ref(0.80)
const clusters = ref<ClusterGroup[]>([])

const smartFolders = ref<SmartFolder[]>([])

const videoTimers = new Map<string, number>();

// Newly added
const incomingFiles = ref<SearchResult[]>([]);
const showIncomingBanner = ref(false);
const incomingCount = computed(() => incomingFiles.value.length);

const fullResultsCache = ref<SearchResult[]>([])


const licenseStatus = ref({ is_pro: false, email: '', limit: 100 })
const showActivateModal = ref(false)
const licenseEmail = ref('')
const licenseKeyInput = ref('')
const activateMsg = ref('')
const activateSuccess = ref(false)

// Variable to store the unlisten function, declared at the top of <script setup>
let unlistenScanProgress: (() => void) | null = null;

// Ensure handleKeydown is defined at the top level of <script setup> so both onMounted and onUnmounted can access it.
function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape' && showActivateModal.value) {
    showActivateModal.value = false;
  }
}

async function checkLicense() {
  try {
    const res: any = await invoke('get_license_status');
    licenseStatus.value = res;
  } catch (e) { console.error(e); }
}

async function submitActivation() {
  if (!licenseEmail.value || !licenseKeyInput.value) return;
  try {
    const resMsg: string = await invoke('activate_pro_license', {
      email: licenseEmail.value,
      licenseKey: licenseKeyInput.value
    });
    activateSuccess.value = true;
    activateMsg.value = resMsg;
    await checkLicense(); // Refresh local license state
    setTimeout(() => { showActivateModal.value = false; }, 1500);
  } catch (err: any) {
    activateSuccess.value = false;
    activateMsg.value = String(err);
  }
}

function acceptIncomingFiles() {
  // Exit search/clustering state
  searchQuery.value = '';
  isImageSearch.value = false;
  searchImagePath.value = '';
  isClusteringView.value = false;

  // Restore full cache (if available)
  if (fullResultsCache.value.length > 0) {
    results.value = [...fullResultsCache.value]
    totalResults.value = results.value.length
  }

  // Insert staged files at the top of results
  incomingFiles.value.forEach(item => {
    if (!results.value.some(r => r.path === item.path)) {
      results.value.unshift(item);
      totalResults.value += 1;
    }
  });
  incomingFiles.value = [];
  showIncomingBanner.value = false;
}

async function loadSmartFolders() {
  try { smartFolders.value = await invoke('get_smart_folders'); } catch (e) { console.error(e); }
}

async function promptSaveSmartFolder() {
  const name = prompt("Enter a name for this Smart Folder:", searchQuery.value);
  if (name) {
    await invoke('save_smart_folder', {
      name, queryText: searchQuery.value,
      useVector: s_vector.value, useOcr: s_ocr.value, useNote: s_note.value, useFilename: s_filename.value
    });
    await loadSmartFolders();
  }
}

async function removeSmartFolder(id: number) {
  await invoke('delete_smart_folder', { id });
  await loadSmartFolders();
}

function applySmartFolder(sf: SmartFolder) {
  searchQuery.value = sf.query_text;
  s_vector.value = sf.use_vector;
  s_ocr.value = sf.use_ocr;
  s_note.value = sf.use_note;
  s_filename.value = sf.use_filename;
  resetAndSearch();
}

function getAssetUrl(path: string) { return convertFileSrc(path) }
function isVideo(path: string) { return path.toLowerCase().match(/\.(mp4|mov|avi|webm|mkv|flv)$/) }

// ============ Score Mapping Utility ============

function mapToHumanScore(rawScore: number, isImage: boolean): number {
  const mapScore = (val: number, minR: number, maxR: number, minH: number, maxH: number) =>
    minH + ((val - minR) / (maxR - minR)) * (maxH - minH);

  let humanScore: number;

  if (isImage) {
    if (rawScore >= 0.84) {
      humanScore = mapScore(rawScore, 0.84, 1.0, 90, 99.9);
    } else if (rawScore >= 0.65) {
      humanScore = mapScore(rawScore, 0.65, 0.84, 60, 89.9);
    } else if (rawScore >= 0.45) {
      humanScore = mapScore(rawScore, 0.45, 0.65, 30, 59.9);
    } else {
      humanScore = mapScore(rawScore, 0.0, 0.45, 1, 29.9);
    }
  } else {
    if (rawScore >= 0.23) {
      humanScore = mapScore(rawScore, 0.23, 0.28, 85, 99.9);
    } else if (rawScore >= 0.20) {
      humanScore = mapScore(rawScore, 0.20, 0.23, 60, 84.9);
    } else if (rawScore >= 0.16) {
      humanScore = mapScore(rawScore, 0.16, 0.20, 30, 59.9);
    } else {
      humanScore = mapScore(rawScore, 0.0, 0.16, 1, 29.9);
    }
  }

  if (humanScore > 99.9) humanScore = 99.9;
  if (humanScore < 0.1) humanScore = 0.1;

  return humanScore;
}

function formatScore(rawScore: number, isImage: boolean = false): string {
  if (rawScore >= 2.0) return '🔥 100.0%';

  const humanScore = mapToHumanScore(rawScore, isImage);

  let icon: string;
  if (humanScore >= 90) icon = '🌟';
  else if (humanScore >= 60) icon = '✅';
  else if (humanScore >= 30) icon = '⚠️';
  else icon = '❌';

  return `${icon} ${humanScore.toFixed(1)}% Match`;
}

function getNumericScore(rawScore: number, isImage: boolean = false): number {
  if (rawScore >= 2.0) return 100.0;
  return mapToHumanScore(rawScore, isImage);
}

function highlight(text: string) {
  if (!text || !searchQuery.value || isImageSearch.value) return text;
  const safeText = text.replace(/</g, '&lt;').replace(/>/g, '&gt;');
  const regex = new RegExp(`(${searchQuery.value})`, 'gi');
  return safeText.replace(regex, '<span class="highlight-text">$1</span>');
}

onMounted(async () => {
  // ---- 1. Restore basic state ----
  folderPath.value = localStorage.getItem('framescout_folder_path') || ''

  // ---- 2. Register keyboard event listener ----
  document.addEventListener('keydown', handleKeydown)

  // ---- 3. Check license status ----
  await checkLicense()

  // ---- 4. Connect to engine ----
  try {
    await invoke('ping_engine')
    engineReady.value = true

    // ---- 5. Load smart folders ----
    await loadSmartFolders()

    // ---- 6. Listen for scan progress (store unlisten function) ----
    const unlisten = await listen('scan-progress', (e: any) => {
      currentStatus.value = e.payload.status
      currentFile.value = e.payload.file_path
      scanProgress.value = { current: e.payload.current, total: e.payload.total }

      if (e.payload.new_files && e.payload.new_files.length > 0) {
        const newItems = e.payload.new_files.map((newPath: string) => ({
          path: newPath,
          timestamp: 0.0,
          score: 2.0,
          matched_tags: ['✨ Fresh Index'],
          ocr_text: '',
          user_note: '',
          isMissing: false,
          expandOcr: false
        }))

        if (searchQuery.value || isImageSearch.value || isClusteringView.value) {
          newItems.forEach((item: typeof newItems[number]) => {
            if (!incomingFiles.value.some(f => f.path === item.path)) {
              incomingFiles.value.push(item)
            }
          })
          showIncomingBanner.value = true
        } else {
          newItems.forEach((item: typeof newItems[number]) => {
            if (!results.value.some(r => r.path === item.path)) {
              results.value.unshift(item)
              totalResults.value += 1
              fullResultsCache.value.unshift(item)
            }
          })
        }
      }
    })
    unlistenScanProgress = unlisten // Save for cleanup on unmount
  } catch (e) {
    console.error("Engine ping failed", e)
  }
})

onUnmounted(() => {
  // ---- Clean up keyboard listener ----
  document.removeEventListener('keydown', handleKeydown)

  // ---- Clean up scan progress listener ----
  if (unlistenScanProgress) {
    unlistenScanProgress()
    unlistenScanProgress = null
  }

  // ---- Clean up video timers ----
  for (const [, timerId] of videoTimers) {
    clearTimeout(timerId)
  }
  videoTimers.clear()
})

function savePath() { localStorage.setItem('framescout_folder_path', folderPath.value) }

async function selectFolder() {
  const selected = await open({ directory: true });
  if (selected) { folderPath.value = selected as string; savePath(); }
}

async function startScan() {
   // Clear staged files left over from the previous scan
  incomingFiles.value = []
  showIncomingBanner.value = false

  if (!folderPath.value) return;
  isScanning.value = true; scanMsg.value = 'Extracting high-dimensional features...'
  try {
    const count: number = await invoke('scan_folder', { folderPath: folderPath.value, scanMode: scanMode.value });
    if (count === 0) {
      scanMsg.value = `✅ Scan completed! No new files found. Memory Matrix is up to date.`;
    } else {
      scanMsg.value = `✅ Memory loaded! Successfully extracted ${count} new spatio-temporal slices.`;
    }
  }
  catch (err) { scanMsg.value = `❌ Failed: ${err}` }
  finally { isScanning.value = false }
}

async function toggleClustering() {
  // Save a full snapshot before entering clustering view
  if (!isClusteringView.value) {
    fullResultsCache.value = [...results.value]
  }
  isClusteringView.value = !isClusteringView.value
  if (isClusteringView.value) {
    await fetchClusters()
  }
}
async function fetchClusters() {
  try {
    const res: any = await invoke('cluster_similar_images', { threshold: clusterThreshold.value });
    clusters.value = res;
  } catch (err) {
    console.error("Clustering error:", err);
  }
}

async function cleanGhosts() {
  if(!confirm("Are you sure you want to purge records of physically deleted files from the database?")) return;
  try {
    const count = await invoke('clean_ghosts');
    alert(`🧹 Successfully purged ${count} ghost records!`);
    // Clear current search state; no need to return to browse-all mode
    // searchQuery.value = '';
    searchImagePath.value = '';
    resetAndSearch();
  } catch(err) { alert("Purge failed: " + err); }
}

async function saveNote(item: any) {
  try { await invoke('update_note', { path: item.path, note: item.user_note }); }
  catch(err) { console.error("Note save failed", err) }
}

async function resetAndSearch() {
  // If currently in browse mode, save a full snapshot
  if (!searchQuery.value && !isImageSearch.value && !isClusteringView.value) {
    fullResultsCache.value = [...results.value]
  }
  isImageSearch.value = false
  currentPage.value = 1
  jumpPage.value = 1
  await performSearch()
}

async function searchByImageAction() {
  // Save current full snapshot (if in browse mode)
  if (!searchQuery.value && !isImageSearch.value && !isClusteringView.value) {
    fullResultsCache.value = [...results.value]
  }
  const selected = await open({
    multiple: false,
    filters: [{ name: 'Images', extensions: ['png', 'jpeg', 'jpg', 'webp'] }]
  });
  if (selected) {
    searchImagePath.value = selected as string;
    isImageSearch.value = true;
    searchQuery.value = `[Visual Mode] Seeking semantic resonance...`;
    currentPage.value = 1; jumpPage.value = 1;
    await performSearch();
  }
}

async function changePage(delta: number) { currentPage.value += delta; jumpPage.value = currentPage.value; await performSearch() }

async function doJump() {
  if (jumpPage.value < 1) jumpPage.value = 1;
  if (jumpPage.value > totalPages.value) jumpPage.value = totalPages.value;
  currentPage.value = jumpPage.value;
  await performSearch()
}

function clearImageSearch() {
  isImageSearch.value = false;
  searchImagePath.value = '';
  searchQuery.value = '';
  results.value = [];
  totalResults.value = 0;
}

async function performSearch() {
  if (!searchQuery.value && !isImageSearch.value) { results.value = []; totalResults.value = 0; return; }
  searchMsg.value = '';

  for (const [_, timerId] of videoTimers) {
    clearTimeout(timerId);
  }
  videoTimers.clear();

  try {
    let response: any = null;
    if (isImageSearch.value) {
      response = await invoke('search_by_image', { imagePath: searchImagePath.value, page: currentPage.value, limit: pageSize.value });
    } else {
      response = await invoke('search_images', {
        text: searchQuery.value, page: currentPage.value, limit: pageSize.value,
        useVector: s_vector.value, useOcr: s_ocr.value, useNote: s_note.value, useFilename: s_filename.value
      });
    }
    let res = response.items;
    totalResults.value = response.total_count;

    res.forEach((item: any) => {
      item.isMissing = false;
      item.expandOcr = false;

      if (isVideo(item.path)) {
        const timerId = window.setTimeout(() => {
          item.isMissing = true;
          videoTimers.delete(item.path);
        }, 25000);
        videoTimers.set(item.path, timerId);
      }
    });
    results.value = res;
  } catch (err) { searchMsg.value = `⚠️ Error: ${err}` }
}

function handleVideoError(item: any) {
  const timerId = videoTimers.get(item.path);
  if (timerId !== undefined) {
    clearTimeout(timerId);
    videoTimers.delete(item.path);
  }
  item.isMissing = true;
}

function handleVideoLoaded(item: any) {
  const timerId = videoTimers.get(item.path);
  if (timerId !== undefined) {
    clearTimeout(timerId);
    videoTimers.delete(item.path);
  }
}

function dismissIncomingBanner() {
  showIncomingBanner.value = false
  // Do not delete incomingFiles; it will be shown again on the next scan
}

</script>

<style>
/* Brand 7-gradient CSS variable mapping */
:root {
  --grad-purple-cyan: linear-gradient(135deg, #8333ff, #67e5e5);
  --grad-purple-blue: linear-gradient(135deg, #821fc6, #2493db);
  --grad-blue-cyan: linear-gradient(135deg, #227ec3, #2fe9e9);
  --grad-champagne: linear-gradient(135deg, #e7eec3, #c3e6ee);
  --grad-gold-yellow: linear-gradient(135deg, #e5a800, #ebff1a);
  --grad-amber-orange: linear-gradient(135deg, #ffee82, #ff8000);
  --grad-yellow-orange: linear-gradient(135deg, #f9f922, #ff6619);

  /* Modal theme variables (Pro default) */
  --modal-border-color: #8333ff;
  --modal-shadow: 0 0 30px rgba(131, 51, 255, 0.3), 0 0 60px rgba(103, 229, 229, 0.1);
  --modal-title-gradient: var(--grad-purple-cyan);
  --modal-btn-primary-bg: var(--grad-purple-cyan);
  --modal-btn-primary-shadow: 0 0 10px rgba(103, 229, 229, 0.3);
  --modal-input-focus-border: #8333ff;
  --modal-input-focus-shadow: 0 0 8px rgba(131, 51, 255, 0.3);
  --modal-success-bg: rgba(103, 229, 229, 0.12);
  --modal-success-color: #67e5e5;
  --modal-success-border: rgba(103, 229, 229, 0.3);

  /* Supplementary error-prompt variables (Pro default) */
  --modal-error-bg: rgba(255, 68, 68, 0.12);
  --modal-error-color: #ff4455;
  --modal-error-border: rgba(255, 68, 68, 0.3);
}

.trial-theme {
  --modal-border-color: #ffaa33;
  --modal-shadow: 0 0 30px rgba(255, 170, 51, 0.3), 0 0 60px rgba(255, 136, 17, 0.1);
  --modal-title-gradient: linear-gradient(135deg, #ffaa33, #ff6619);
  --modal-btn-primary-bg: linear-gradient(135deg, #ffaa33, #ff6619);
  --modal-btn-primary-shadow: 0 0 10px rgba(255, 166, 77, 0.3);
  --modal-input-focus-border: #ffaa33;
  --modal-input-focus-shadow: 0 0 8px rgba(255, 170, 51, 0.3);
  --modal-success-bg: rgba(255, 204, 102, 0.12);
  --modal-success-color: #ffaa33;
  --modal-success-border: rgba(255, 204, 102, 0.3);
}

@keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }

@keyframes pulse {
  0%, 100% { transform: scale(1); opacity: 0.9; }
  50% { transform: scale(1.08); opacity: 1; }
}

body { margin: 0; background-color: #0a0a0c; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }

/* Splash Screen */
.neural-splash {
  position: fixed; top:0; left:0; width:100vw; height:100vh; background:#070709; z-index:9999;
  display:flex; flex-direction:column; align-items:center; justify-content:center; color:#67e5e5; font-family:monospace;
}
.loader-ring {
  border: 4px solid #1a1a24; border-top: 4px solid #8333ff; border-radius: 50%; width: 50px; height: 50px;
  animation: spin 1s linear infinite; margin-bottom: 20px; box-shadow: 0 0 15px rgba(131,51,255,0.5);
}
.splash-title { background: var(--grad-purple-cyan); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; }
.splash-subtitle { color: #888; font-size: 14px; }

/* Main Layout */
.app-container { padding: 40px; color: #fff; min-height: 100vh; max-width: 1400px; margin: 0 auto; }
.brand-header { text-align: center; font-size: 36px; font-weight: 800; background: var(--grad-purple-cyan); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; }

/* Common Button Group */
.top-action-bar { display: flex; justify-content: center; gap: 10px; flex-wrap: wrap; margin-bottom: 20px; }
.btn { padding: 10px 18px; border-radius: 6px; border: none; font-weight: bold; cursor: pointer; transition: all 0.2s ease; }
.btn-primary { background: var(--grad-purple-cyan); color: #000; box-shadow: 0 0 10px rgba(103,229,229,0.3); }
.btn-primary:hover { opacity: 0.95; transform: translateY(-1px); }
.btn-secondary { background: #1e1e26; color: #fff; border: 1px solid #333344; }
.btn-cluster { background: var(--grad-amber-orange); color: #000; }
.btn-danger { background: #ff3344; color: #fff; }
.btn-outline-primary { background: transparent; border: 1px solid #67e5e5; color: #67e5e5; }
.btn-sm { padding: 5px 10px; font-size: 12px; }

/* Input Fields */
.custom-input { padding: 10px 15px; border-radius: 6px; border: 1px solid #333344; background: #121218; color: #fff; outline: none; }
.path-input { width: 320px; }
.custom-select { padding: 10px; background: #121218; color: #fff; border-radius: 6px; border: 1px solid #333344; }

/* Monitor Bus */
.monitor-wrapper { text-align: center; display: flex; flex-direction: column; align-items: center; margin-bottom: 25px; }
.scan-msg { color: #ebff1a; font-weight: bold; margin-bottom: 10px; }
.bus-panel { width: 100%; max-width: 700px; background: #0c0c10; padding: 15px; border-radius: 8px; border: 1px solid #222233; text-align: left; font-family: monospace; }
.bus-header { display: flex; justify-content: space-between; border-bottom: 1px solid #1a1a24; padding-bottom: 8px; }
.bus-title { background: var(--grad-purple-blue); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; font-weight: bold; }
.bus-status { color: #2fe9e9; }
.status-warning { color: #ffaa00; }
.progress-track { width: 100%; background: #1a1a24; border-radius: 4px; height: 8px; overflow: hidden; margin-top: 5px; }
.progress-fill { background: var(--grad-blue-cyan); height: 100%; transition: width 0.2s ease; }
.current-file-text { color: #777; font-size: 12px; margin-top: 8px; word-break: break-all; }

/* Search Console */
.search-console { background: #12121a; padding: 25px; border-radius: 12px; max-width: 800px; margin: 0 auto 30px auto; border: 1px solid #222233; }
.search-options { display: flex; justify-content: center; align-items: center; gap: 15px; color: #aaa; font-size: 13px; margin-bottom: 15px; }
.option-divider { width: 1px; height: 16px; background: #333; }
.search-bar { display: flex; justify-content: center; gap: 10px; }
.search-input { width: 100%; max-width: 480px; font-size: 16px; border-color: #8333ff; }

/* Result Card Grid */
.results-grid { display: flex; flex-wrap: wrap; gap: 20px; justify-content: center; }
.result-card { background: #14141e; padding: 15px; border-radius: 10px; width: 340px; border: 1px solid #222233; display: flex; flex-direction: column; transition: transform 0.2s; }
.result-card:hover { transform: translateY(-3px); border-color: #8333ff; }
.media-preview { width: 100%; height: 200px; border-radius: 6px; object-fit: cover; background: #000; }
.card-content { margin-top: 10px; flex-grow: 1; display: flex; flex-direction: column; }
.file-path { font-size: 12px; color: #888; word-break: break-all; margin: 0 0 10px 0; }
.card-meta-row { display: flex; justify-content: space-between; align-items: center; }
.match-score { font-size: 14px; font-weight: bold; background: var(--grad-yellow-orange); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; }

/* Tag Group */
.tags-wrapper { display: flex; gap: 6px; flex-wrap: wrap; margin: 8px 0; }
.tag-badge { font-size: 11px; padding: 3px 6px; border-radius: 4px; background: rgba(34,126,195,0.2); color: #2fe9e9; border: 1px solid rgba(47,233,233,0.3); }
.semantic-tag { background: rgba(131,51,255,0.2); color: #67e5e5; border-color: #8333ff; }
.low-tag { background: rgba(255,255,255,0.05); color: #666; border-color: #333; }
.highlight-text { background: var(--grad-yellow-orange); color: #000; font-weight: bold; padding: 0 2px; border-radius: 2px; }

/* Panel Regions */
.ocr-panel, .note-panel { margin-top: 10px; background: #0a0a0e; padding: 8px; border-radius: 6px; border: 1px solid #1a1a24; }
.ocr-label, .note-label { font-size: 11px; color: #666; margin: 0 0 4px 0; }
.ocr-content { font-size: 12px; color: #ccc; margin: 0; line-height: 1.4; }
.ocr-collapsed { max-height: 38px; overflow: hidden; }
.custom-textarea { width: 100%; height: 50px; background: #121218; color: #fff; border: 1px solid #222233; border-radius: 4px; padding: 6px; font-size: 12px; box-sizing: border-box; resize: vertical; }

/* Visual Clustering Panel */
.clustering-container { background: #12121a; padding: 25px; border-radius: 12px; border: 1px solid #8333ff; }
.clustering-header { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #222233; padding-bottom: 15px; margin-bottom: 20px; }
.cluster-controls { display: flex; align-items: center; gap: 10px; color: #aaa; }
.cluster-grid { display: flex; flex-direction: column; gap: 20px; }
.cluster-card { background: #0a0a0e; padding: 15px; border-radius: 8px; border: 1px solid #222233; }
.cluster-badge { font-weight: bold; background: var(--grad-amber-orange); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 12px; }
.cluster-thumbnails { display: flex; gap: 10px; overflow-x: auto; padding-bottom: 5px; }
.cluster-thumbnails::-webkit-scrollbar { height: 6px; }
.cluster-thumbnails::-webkit-scrollbar-thumb { background: #333; border-radius: 3px; }
.cluster-thumbnails::-webkit-scrollbar-track { background: transparent; }
.cluster-thumb { width: 120px; height: 120px; object-fit: cover; border-radius: 6px; border: 1px solid #333; }

/* Pagination */
.pagination-wrapper { display: flex; justify-content: center; align-items: center; gap: 15px; margin-top: 40px; padding-bottom: 40px; }
.page-current { font-weight: bold; background: var(--grad-purple-cyan); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent; }
.jump-input { width: 45px; padding: 5px; text-align: center; background: #121218; border: 1px solid #333; color: #fff; border-radius: 4px; }

button:disabled {
  opacity: 0.5;
  cursor: not-allowed !important;
}

/* Header Layout */
.brand-header-box {
  display: flex;
  align-items: center;
  justify-content: center;  /* center title */
  gap: 12px;
  margin-bottom: 20px;
  position: relative; /* optional, for positioning */
}

/* Smart Folders */
.smart-folders-bar {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  margin-bottom: 20px;
  flex-wrap: wrap;
}
.smart-folder-label { font-size: 12px; color: #888; font-weight: bold; }
.smart-folder-pill {
  background: #14141f; border: 1px solid #333348; padding: 4px 10px; border-radius: 20px;
  font-size: 12px; cursor: pointer; display: flex; align-items: center; gap: 6px; transition: all 0.2s;
}
.smart-folder-pill:hover { border-color: #67e5e5; background: #1a1a2b; }
.sf-delete { color: #666; font-size: 10px; border-radius: 50%; padding: 0 4px; }
.sf-delete:hover { color: #ff4444; background: rgba(255,68,68,0.2); }

/* Empty Cluster Hint */
.empty-clusters {
  text-align: center; color: #888; padding: 30px 20px; font-size: 14px;
  background: rgba(255,255,255,0.02); border-radius: 8px; border: 1px dashed #333;
}
.empty-hint { font-size: 12px; color: #666; margin-top: 8px; }

/* Missing File Card */
.missing-card {
  display: flex; align-items: center; justify-content: center; height: 200px;
  background: #1a1a24; border-radius: 6px; color: #666; font-size: 16px; gap: 10px; flex-direction: column;
}
.missing-icon { font-size: 40px; opacity: 0.5; }

/* Error Message */
.error-msg {
  color: #ff4444; background: rgba(255,68,68,0.1); border: 1px solid rgba(255,68,68,0.3);
  padding: 8px 12px; border-radius: 6px; font-size: 13px; text-align: center; margin-top: 10px;
}

/* Logo Animation */
.logo-wrapper {
  animation: pulse 2s infinite ease-in-out;
}

/* New File Banner */
.incoming-banner {
  position: sticky;
  top: 0;
  z-index: 10;
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: var(--grad-purple-cyan);
  color: #000;
  padding: 12px 20px;
  border-radius: 8px;
  cursor: pointer;
  font-weight: bold;
  margin: 10px auto;
  max-width: 95%;
  width: fit-content;
  box-shadow: 0 0 15px rgba(131,51,255,0.4);
  transition: transform 0.2s ease, box-shadow 0.2s ease;
  animation: slideDown 0.3s ease-out;
}
.incoming-banner:hover {
  transform: scale(1.03);
  box-shadow: 0 0 25px rgba(131,51,255,0.6);
}
.banner-close {
  cursor: pointer;
  font-size: 16px;
  opacity: 0.7;
  margin-left: 10px;
}
.banner-close:hover {
  opacity: 1;
}
@keyframes slideDown {
  from { transform: translateY(-20px); opacity: 0; }
  to { transform: translateY(0); opacity: 1; }
}

/* Pro Activation Modal */
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.75);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  animation: fadeIn 0.2s ease;
}

.modal-card {
  background: #12121a;
  border: 1px solid var(--modal-border-color);
  border-radius: 16px;
  padding: 32px 28px;
  max-width: 460px;
  width: 90%;
  box-shadow: var(--modal-shadow);
  color: #ddd;
  position: relative;
  animation: scaleIn 0.25s ease;
}

.modal-card h2 {
  font-size: 22px;
  font-weight: 800;
  background: var(--modal-title-gradient);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  margin: 0 0 8px 0;
  text-align: center;
}

.modal-sub {
  font-size: 13px;
  color: #888;
  text-align: center;
  margin: 0 0 24px 0;
  line-height: 1.4;
}

.form-group {
  margin-bottom: 18px;
}

.form-group label {
  display: block;
  font-size: 12px;
  color: #aaa;
  margin-bottom: 6px;
  font-weight: 600;
  letter-spacing: 0.5px;
}

/* Primary button inside modal */
.modal-card .btn-primary {
  background: var(--modal-btn-primary-bg);
  color: #000;
  box-shadow: var(--modal-btn-primary-shadow);
  border: none;
}

.modal-card .btn-primary:hover {
  opacity: 0.95;
  transform: translateY(-1px);
}

.modal-input,
.modal-textarea {
  width: 100%;
  box-sizing: border-box;
  background: #0c0c12;
  border: 1px solid #2a2a3a;
  border-radius: 8px;
  padding: 12px 14px;
  color: #eee;
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s, box-shadow 0.2s;
}

.modal-input:focus,
.modal-textarea:focus {
  border-color: var(--modal-input-focus-border);
  box-shadow: var(--modal-input-focus-shadow);
}

.modal-textarea {
  min-height: 70px;
  resize: vertical;
  font-family: monospace;
  font-size: 13px;
}

.activate-msg {
  text-align: center;
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  margin: 16px 0 0 0;
}

 .msg-success {
  background: var(--modal-success-bg);
  color: var(--modal-success-color);
  border: 1px solid var(--modal-success-border);
}

.msg-error {
  background: var(--modal-error-bg);
  color: var(--modal-error-color);
  border: 1px solid var(--modal-error-border);
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}

.modal-actions .btn {
  padding: 10px 24px;
  font-size: 14px;
  border-radius: 8px;
}

/* Animations */
@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes scaleIn {
  from { transform: scale(0.92); opacity: 0; }
  to { transform: scale(1); opacity: 1; }
}


/* License Status Button */
.license-badge {
  display: inline-flex;
  align-items: center;
  cursor: pointer;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.3px;
  transition: all 0.25s ease;
  user-select: none;
  white-space: nowrap;
  margin-left: auto; /* right-align */
  padding: 0;
  border-radius: 0;
  background: none;
  border: none;
}

.badge-pro {
  background: linear-gradient(135deg, #8333ff, #67e5e5);
  color: #000;
  box-shadow: 0 0 12px rgba(131, 51, 255, 0.4);
  border: 1px solid rgba(131, 51, 255, 0.6);
  border-radius: 999px;   /* large value ensures pill shape always */
  padding: 6px 20px;      /* horizontal padding to widen the button */
  font-size: 13px;        /* slightly smaller font to prevent overflow */
  line-height: 1.4;       /* vertical centering */
  display: inline-block;  /* ensure padding takes effect */
}

.badge-pro:hover {
  transform: scale(1.05);
  box-shadow: 0 0 20px rgba(131, 51, 255, 0.6);
}

.badge-trial {
  background: #1e1e2a;
  color: #ffaa33;
  border: 1px solid #ffaa33;
  box-shadow: 0 0 8px rgba(255, 170, 51, 0.2);
  animation: trialPulse 3s infinite ease-in-out;
  border-radius: 999px;   /* large value ensures pill shape always */
  padding: 6px 20px;      /* horizontal padding to widen the button */
  font-size: 13px;        /* slightly smaller font to prevent overflow */
  line-height: 1.4;       /* vertical centering */
  display: inline-block;  /* ensure padding takes effect */
}

.badge-trial:hover {
  background: #2a2a3a;
  border-color: #ffbb44;
  box-shadow: 0 0 14px rgba(255, 170, 51, 0.4);
  transform: scale(1.05);
}

@keyframes trialPulse {
  0%, 100% { box-shadow: 0 0 8px rgba(255, 170, 51, 0.2); }
  50% { box-shadow: 0 0 16px rgba(255, 170, 51, 0.5); }
}

@media (max-width: 768px) {
  .brand-header-box {
    flex-wrap: wrap;
    justify-content: center;
  }
  .license-badge {
    margin-left: 0;
    margin-top: 8px;
  }
}

</style>
