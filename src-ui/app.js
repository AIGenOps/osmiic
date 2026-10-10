document.addEventListener('DOMContentLoaded', async () => {
    // Toast Notification Manager
    function showToast(message, type = 'info') {
        const container = document.getElementById('toastContainer');
        if (!container) return;

        const toast = document.createElement('div');
        toast.className = `toast ${type}`;
        toast.innerHTML = `
            <i class="fa-solid ${type === 'error' ? 'fa-circle-exclamation' : (type === 'success' ? 'fa-circle-check' : 'fa-circle-info')}"></i>
            <span>${escapeHtml(message)}</span>
        `;

        container.appendChild(toast);

        setTimeout(() => {
            toast.style.opacity = '0';
            toast.style.transition = 'opacity 0.3s ease';
            setTimeout(() => toast.remove(), 300);
        }, 4500);
    }

    // Modal Manager
    const errorModal = document.getElementById('errorModal');
    const modalErrorContent = document.getElementById('modalErrorContent');
    const closeModalBtn = document.getElementById('closeModalBtn');

    if (closeModalBtn && errorModal) {
        closeModalBtn.addEventListener('click', () => {
            errorModal.classList.add('hidden');
        });
    }

    function showDiagnosticModal(details) {
        if (!errorModal || !modalErrorContent) return;
        modalErrorContent.textContent = details || 'No details recorded.';
        errorModal.classList.remove('hidden');
    }

    // Navigation Views Switcher
    const navItems = document.querySelectorAll('.nav-item');
    const viewPanels = document.querySelectorAll('.view-panel');

    navItems.forEach(btn => {
        btn.addEventListener('click', () => {
            navItems.forEach(n => n.classList.remove('active'));
            viewPanels.forEach(p => p.classList.remove('active'));

            btn.classList.add('active');
            const viewId = btn.getAttribute('data-view');
            const targetView = document.getElementById(viewId);
            if (targetView) targetView.classList.add('active');

            if (viewId === 'libraryView') {
                loadLibrary();
            } else if (viewId === 'settingsView') {
                loadSettings();
            }
        });
    });

    // Form Elements
    const urlInput = document.getElementById('urlInput');
    const previewBtn = document.getElementById('previewBtn');
    const urlForm = document.getElementById('urlForm');
    const previewCard = document.getElementById('previewCard');
    const prevThumb = document.getElementById('prevThumb');
    const prevTitle = document.getElementById('prevTitle');
    const prevArtist = document.getElementById('prevArtist');
    const prevDuration = document.getElementById('prevDuration');
    const prevLangBadge = document.getElementById('prevLangBadge');
    const categorySelect = document.getElementById('categorySelect');
    const modeSelect = document.getElementById('modeSelect');
    const formatSelect = document.getElementById('formatSelect');

    // Queue Elements
    const queueContainer = document.getElementById('queueContainer');
    const clearCompletedBtn = document.getElementById('clearCompletedBtn');
    const startAllBtn = document.getElementById('startAllBtn');

    // Library Elements
    const refreshLibBtn = document.getElementById('refreshLibBtn');
    const musicLibraryList = document.getElementById('musicLibraryList');
    const videoLibraryList = document.getElementById('videoLibraryList');
    const libSearchInput = document.getElementById('libSearchInput');
    const libSortSelect = document.getElementById('libSortSelect');
    const libLangFilter = document.getElementById('libLangFilter');

    // Settings Elements
    const settingsForm = document.getElementById('settingsForm');
    const settingDownloadDir = document.getElementById('settingDownloadDir');
    const settingDefaultMode = document.getElementById('settingDefaultMode');
    const settingAudioFormat = document.getElementById('settingAudioFormat');
    const settingVideoFormat = document.getElementById('settingVideoFormat');
    const settingDefaultCategory = document.getElementById('settingDefaultCategory');
    const settingAutoEmbed = document.getElementById('settingAutoEmbed');
    const settingAutoOpen = document.getElementById('settingAutoOpen');
    const settingMaxConcurrent = document.getElementById('settingMaxConcurrent');
    const settingCookiesBrowser = document.getElementById('settingCookiesBrowser');
    const settingDuplicateAction = document.getElementById('settingDuplicateAction');
    const settingSubLangs = document.getElementById('settingSubLangs');
    const settingDownloadSubs = document.getElementById('settingDownloadSubs');
    const settingAutoLyrics = document.getElementById('settingAutoLyrics');
    const resetSettingsBtn = document.getElementById('resetSettingsBtn');
    const systemVersion = document.getElementById('systemVersion');

    // Playlist Modal Elements
    const parsePlaylistBtn = document.getElementById('parsePlaylistBtn');
    const playlistModal = document.getElementById('playlistModal');
    const closePlaylistModalBtn = document.getElementById('closePlaylistModalBtn');
    const playlistTracksList = document.getElementById('playlistTracksList');
    const playlistSelectAll = document.getElementById('playlistSelectAll');
    const playlistMasterCategory = document.getElementById('playlistMasterCategory');
    const queueSelectedPlaylistBtn = document.getElementById('queueSelectedPlaylistBtn');
    const selectedTrackCount = document.getElementById('selectedTrackCount');

    // Tag Editor Modal Elements
    const tagEditorModal = document.getElementById('tagEditorModal');
    const closeTagModalBtn = document.getElementById('closeTagModalBtn');
    const tagEditorForm = document.getElementById('tagEditorForm');
    const editFilePath = document.getElementById('editFilePath');
    const editTitle = document.getElementById('editTitle');
    const editArtist = document.getElementById('editArtist');
    const editCategory = document.getElementById('editCategory');
    const editCoverPath = document.getElementById('editCoverPath');
    const editLyrics = document.getElementById('editLyrics');
    const fetchLyricsBtn = document.getElementById('fetchLyricsBtn');

    let currentMetadata = null;
    let downloadQueue = [];
    let appSettings = null;
    let parsedPlaylistTracks = [];

    // Helper: Tauri IPC Invocation Bridge
    async function invokeIPC(cmd, args = {}) {
        if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke) {
            return await window.__TAURI__.core.invoke(cmd, args);
        }
        if (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke) {
            return await window.__TAURI_INTERNALS__.invoke(cmd, args);
        }
        if (window.__TAURI__ && window.__TAURI__.invoke) {
            return await window.__TAURI__.invoke(cmd, args);
        }
        throw new Error('Tauri IPC API not initialized');
    }

    // System Info Load
    async function loadSystemInfo() {
        try {
            const info = await invokeIPC('get_system_info');
            if (systemVersion && info) {
                systemVersion.textContent = `OSMIIC v${info.app_version} | yt-dlp ${info.ytdlp_version.split(' ')[0]}`;
            }
            const ytdlpBadge = document.getElementById('ytdlpVersionBadge');
            if (ytdlpBadge && info && info.ytdlp_version) {
                ytdlpBadge.textContent = `yt-dlp v${info.ytdlp_version.split(' ')[0]}`;
            }

            if (info) {
                if (info.ytdlp_version.includes('Not installed')) {
                    showToast('Dependency Warning: yt-dlp binary is missing or not found in system PATH.', 'error');
                }
                if (info.ffmpeg_version.includes('Missing')) {
                    showToast('Dependency Warning: FFmpeg is missing. Audio conversion & metadata tagging require FFmpeg.', 'error');
                }
            }
        } catch (e) {
            console.log('System info load fallback');
        }
    }

    const updateYtdlpBtn = document.getElementById('updateYtdlpBtn');
    if (updateYtdlpBtn) {
        updateYtdlpBtn.addEventListener('click', async () => {
            updateYtdlpBtn.disabled = true;
            updateYtdlpBtn.innerHTML = '<i class="fa-solid fa-spinner fa-spin"></i> Checking...';
            showToast('Checking for yt-dlp update...', 'info');

            try {
                const res = await invokeIPC('update_ytdlp');
                showToast(res || 'yt-dlp update check completed', 'info');
                await loadSystemInfo();
            } catch (err) {
                showToast(`yt-dlp update error: ${err}`, 'error');
            } finally {
                updateYtdlpBtn.disabled = false;
                updateYtdlpBtn.innerHTML = '<i class="fa-solid fa-arrows-rotate"></i> Check / Update yt-dlp (-U)';
            }
        });
    }

    // Dynamic Format Switcher based on Mode (Audio vs Video)
    modeSelect.addEventListener('change', () => {
        const isAudio = modeSelect.value === 'audio';
        formatSelect.innerHTML = isAudio ? `
            <option value="mp3" selected>MP3 (320kbps)</option>
            <option value="flac">FLAC (Lossless)</option>
            <option value="m4a">M4A (AAC)</option>
        ` : `
            <option value="mp4" selected>MP4 (4K / 1080p Video)</option>
            <option value="mkv">MKV (Video)</option>
            <option value="webm">WEBM (Video)</option>
        `;
    });

    // Settings Handler
    async function loadSettings() {
        try {
            appSettings = await invokeIPC('get_settings');
            if (appSettings) {
                if (settingDownloadDir) settingDownloadDir.value = appSettings.download_dir;
                if (settingDefaultMode) settingDefaultMode.value = appSettings.default_mode;
                if (settingAudioFormat) settingAudioFormat.value = appSettings.default_audio_format;
                if (settingVideoFormat) settingVideoFormat.value = appSettings.default_video_format;
                if (settingDefaultCategory) settingDefaultCategory.value = appSettings.default_category;
                if (settingAutoEmbed) settingAutoEmbed.checked = appSettings.auto_embed;
                if (settingAutoOpen) settingAutoOpen.checked = appSettings.auto_open;
                if (settingMaxConcurrent) settingMaxConcurrent.value = appSettings.max_concurrent_downloads || 3;
                if (settingCookiesBrowser) settingCookiesBrowser.value = appSettings.cookies_from_browser || 'none';
                if (settingDuplicateAction) settingDuplicateAction.value = appSettings.duplicate_action || 'skip';
                if (settingSubLangs) settingSubLangs.value = appSettings.subtitle_languages || 'en';
                if (settingDownloadSubs) settingDownloadSubs.checked = appSettings.download_subtitles || false;
                if (settingAutoLyrics) settingAutoLyrics.checked = appSettings.auto_fetch_lyrics !== false;
            }
        } catch (e) {
            console.error('Settings load error:', e);
        }
    }

    if (settingsForm) {
        settingsForm.addEventListener('submit', async (e) => {
            e.preventDefault();
            const maxVal = settingMaxConcurrent ? parseInt(settingMaxConcurrent.value, 10) : 3;
            const newSettings = {
                download_dir: settingDownloadDir.value.trim(),
                default_mode: settingDefaultMode.value,
                default_audio_format: settingAudioFormat.value,
                default_video_format: settingVideoFormat.value,
                default_category: settingDefaultCategory.value,
                auto_embed: settingAutoEmbed.checked,
                auto_open: settingAutoOpen.checked,
                max_concurrent_downloads: isNaN(maxVal) ? 3 : Math.min(10, Math.max(1, maxVal)),
                cookies_from_browser: settingCookiesBrowser ? settingCookiesBrowser.value : 'none',
                cookies_file_path: null,
                duplicate_action: settingDuplicateAction ? settingDuplicateAction.value : 'skip',
                subtitle_languages: settingSubLangs ? settingSubLangs.value.trim() : 'en',
                download_subtitles: settingDownloadSubs ? settingDownloadSubs.checked : false,
                auto_fetch_lyrics: settingAutoLyrics ? settingAutoLyrics.checked : true
            };

            try {
                await invokeIPC('save_settings', { settings: newSettings });
                appSettings = newSettings;
                showToast('Settings saved successfully', 'success');
            } catch (err) {
                showToast(`Failed to save settings: ${err}`, 'error');
            }
        });
    }

    if (resetSettingsBtn) {
        resetSettingsBtn.addEventListener('click', async () => {
            const defaults = {
                download_dir: '',
                default_mode: 'audio',
                default_audio_format: 'mp3',
                default_video_format: 'mp4',
                default_category: 'Auto',
                auto_embed: true,
                auto_open: false,
                max_concurrent_downloads: 3,
                cookies_from_browser: 'none',
                cookies_file_path: null,
                duplicate_action: 'skip',
                subtitle_languages: 'en',
                download_subtitles: false,
                auto_fetch_lyrics: true
            };
            try {
                await invokeIPC('save_settings', { settings: defaults });
                await loadSettings();
                showToast('Settings reset to default values', 'info');
            } catch (e) {
                showToast('Could not reset settings', 'error');
            }
        });
    }

    // Persistence: LocalStorage Queue State Handler
    function saveQueue() {
        try {
            localStorage.setItem('osmiic_queue_v1', JSON.stringify(downloadQueue));
        } catch (e) {
            console.error('Failed to save queue state:', e);
        }
    }

    function loadQueue() {
        try {
            const stored = localStorage.getItem('osmiic_queue_v1');
            if (stored) {
                downloadQueue = JSON.parse(stored);
                // Reset any interrupted progress states back to 'Queued'
                downloadQueue.forEach(item => {
                    if (item.status.includes('Downloading') || item.status === 'Queuing...' || item.status.includes('Waiting') || item.status.includes('Processing')) {
                        item.status = 'Queued';
                        item.percent = 0;
                    }
                });
                renderQueue();
            }
        } catch (e) {
            console.error('Failed to load queue state:', e);
        }
    }

    // Register Tauri IPC Real-Time Event Listeners
    async function setupEventListeners() {
        const listenFn = (window.__TAURI__ && window.__TAURI__.event && window.__TAURI__.event.listen)
            || (window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.listen);

        if (listenFn) {
            try {
                await listenFn('download-progress', (event) => {
                    const data = (event && event.payload !== undefined) ? event.payload : event;
                    if (!data || !data.id) return;

                    const target = downloadQueue.find(i => i.id === data.id);
                    if (target) {
                        if (data.status === 'queued') {
                            target.status = 'Waiting for slot...';
                            target.percent = 0;
                            target.speed = '';
                            target.eta = 'Queued';
                        } else if (data.status === 'processing') {
                            target.status = 'Processing Metadata...';
                            target.percent = Math.max(1, Math.round(data.percent || 1));
                            target.speed = data.speed || '';
                            target.eta = data.eta || '';
                        } else {
                            target.percent = Math.max(1, Math.round(data.percent || 1));
                            target.speed = data.speed || '';
                            target.eta = data.eta || '';
                            target.status = `Downloading (${target.percent}%)`;
                        }
                        saveQueue();
                        renderQueue();
                    }
                });

                await listenFn('download-complete', (event) => {
                    const data = (event && event.payload !== undefined) ? event.payload : event;
                    if (!data || !data.id) return;

                    const target = downloadQueue.find(i => i.id === data.id);
                    if (target) {
                        target.percent = 100;
                        target.status = 'Completed';
                        target.filePath = data.file_path;
                        saveQueue();
                        renderQueue();
                        showToast(`Downloaded: ${target.title}`, 'success');
                        sendDesktopNotification('OSMIIC Download Complete', target.title);

                        if (appSettings && appSettings.auto_open && data.file_path) {
                            invokeIPC('open_folder', { path: data.file_path }).catch(() => {});
                        }
                    }
                    loadLibrary();
                    processQueueNext();
                });

                await listenFn('download-error', (event) => {
                    const data = (event && event.payload !== undefined) ? event.payload : event;
                    if (!data || !data.id) return;

                    const target = downloadQueue.find(i => i.id === data.id);
                    if (target) {
                        target.status = data.is_cancelled ? 'Cancelled' : ('Error: ' + (data.error || 'Failed'));
                        target.errorDetails = data.error || '';
                        saveQueue();
                        renderQueue();
                        showToast(data.is_cancelled ? 'Download cancelled' : `Download failed: ${data.error || 'Unknown error'}`, data.is_cancelled ? 'info' : 'error');
                    }
                    processQueueNext();
                });
            } catch (e) {
                console.error('Event listener attach error:', e);
            }
        }
    }

    await setupEventListeners();

    async function processQueueNext() {
        const nextItem = downloadQueue.find(i => i.status === 'Queued');
        if (nextItem) {
            await startSingleDownload(nextItem);
            processQueueNext();
        }
    }

    // 1. Instant Metadata Preview
    previewBtn.addEventListener('click', async () => {
        const url = urlInput.value.trim();
        if (!url) {
            showToast('Please enter a YouTube or media URL first', 'error');
            return;
        }

        previewBtn.disabled = true;
        previewBtn.innerHTML = '<i class="fa-solid fa-spinner fa-spin"></i> Fetching...';

        try {
            const meta = await invokeIPC('get_metadata', {
                url,
                language: categorySelect.value
            });

            currentMetadata = meta;
            prevThumb.src = meta.thumbnail || 'https://via.placeholder.com/150';
            prevTitle.textContent = meta.title;
            prevArtist.innerHTML = `<i class="fa-solid fa-user-ninja"></i> ${escapeHtml(meta.artist)}`;

            const durNum = Number(meta.duration) || 0;
            const mins = Math.floor(durNum / 60);
            const secs = String(durNum % 60).padStart(2, '0');
            prevDuration.innerHTML = `<i class="fa-regular fa-clock"></i> ${mins}:${secs}`;
            prevLangBadge.textContent = meta.language;

            if (categorySelect.value === 'Auto') {
                categorySelect.value = meta.language;
            }

            previewCard.classList.remove('hidden');
            showToast(`Metadata resolved: ${meta.title}`, 'success');
        } catch (err) {
            showToast(`Failed to fetch metadata: ${err}`, 'error');
        } finally {
            previewBtn.disabled = false;
            previewBtn.innerHTML = '<i class="fa-solid fa-eye"></i> Preview';
        }
    });

    // 2. Add to Queue Form Handler
    urlForm.addEventListener('submit', (e) => {
        e.preventDefault();
        const url = urlInput.value.trim();
        if (!url) return;

        const queueId = 'dl_' + Date.now() + '_' + Math.random().toString(36).substr(2, 4);
        const title = currentMetadata ? currentMetadata.title : url;
        const thumb = currentMetadata ? currentMetadata.thumbnail : '';
        const selectedCategory = categorySelect.value;

        const item = {
            id: queueId,
            url,
            title,
            thumb,
            mode: modeSelect.value,
            format: formatSelect.value,
            category: selectedCategory,
            status: 'Queued',
            percent: 0,
            speed: '',
            eta: '',
            filePath: null,
            errorDetails: null
        };

        downloadQueue.push(item);
        saveQueue();
        renderQueue();

        showToast('Added to queue.', 'success');

        urlInput.value = '';
        previewCard.classList.add('hidden');
        currentMetadata = null;

        // Trigger queue processor respecting concurrency limit
        processQueueNext();
    });

    function renderQueue() {
        if (!queueContainer) return;
        queueContainer.innerHTML = '';

        if (downloadQueue.length === 0) {
            queueContainer.innerHTML = `
                <div class="empty-queue">
                    <i class="fa-solid fa-layer-group queue-icon"></i>
                    <p>No downloads in queue. Paste a URL above to start!</p>
                </div>`;
            return;
        }

        downloadQueue.slice().reverse().forEach(item => {
            const el = document.createElement('div');
            el.id = item.id;
            el.className = 'queue-item';

            const isCompleted = item.status.includes('Completed');
            const isCancelled = item.status.includes('Cancelled');
            const isError = item.status.startsWith('Error');
            const isWaiting = item.status === 'Waiting for slot...' || item.status.includes('Waiting');
            const isDownloading = item.status.includes('Downloading') || item.status.includes('Processing');
            const isCancelable = isDownloading || isWaiting;

            const statusColor = isError ? 'var(--danger)' : (isCancelled ? 'var(--signal)' : (isCompleted ? 'var(--tape)' : (isWaiting ? 'var(--paper-faint)' : 'var(--signal)')));

            el.innerHTML = `
                <img src="${item.thumb || 'https://via.placeholder.com/50'}" class="queue-thumb" alt="Cover" onerror="this.src='https://via.placeholder.com/50'">
                <div class="queue-details">
                    <div class="queue-title" title="${escapeHtml(item.title)}">${escapeHtml(item.title)}</div>
                    <div class="queue-sub">
                        <span class="category-tag"><i class="fa-solid fa-folder"></i> Folder: ${item.category}</span>
                        <span>Mode: ${item.mode.toUpperCase()} (${item.format})</span>
                        ${item.speed ? `<span>• ${item.speed}</span>` : ''}
                    </div>
                    <div class="progress-bar-bg">
                        <div class="progress-bar-fill" style="width: ${item.percent}%; background-color: ${statusColor};"></div>
                    </div>
                </div>
                <div class="queue-status-box">
                    <span style="font-size: 0.75rem; font-weight: 600; color: ${statusColor}; ${isCompleted ? 'class="done"' : ''}">${escapeHtml(item.status)}</span>
                    <div class="queue-item-actions">
                        ${isCancelable ? `<button class="btn-item-action cancel-btn" data-action="cancel" data-id="${item.id}" title="Cancel Download"><i class="fa-solid fa-stop"></i></button>` : ''}
                        ${isCompleted && item.filePath ? `<button class="btn-item-action open-btn" data-action="open" data-id="${item.id}" title="Open Folder"><i class="fa-solid fa-folder-open"></i></button>` : ''}
                        ${isError || isCancelled || item.status === 'Queued' ? `<button class="btn-item-action retry-btn" data-action="retry" data-id="${item.id}" title="Retry Download"><i class="fa-solid fa-play"></i></button>` : ''}
                        ${item.errorDetails ? `<button class="btn-item-action info-btn" data-action="details" data-id="${item.id}" title="View Diagnostics"><i class="fa-solid fa-code"></i></button>` : ''}
                        <button class="btn-item-action delete-btn" data-action="delete" data-id="${item.id}" title="Remove Item"><i class="fa-solid fa-trash-can"></i></button>
                    </div>
                </div>
            `;

            queueContainer.appendChild(el);
        });

        // Attach action handlers
        queueContainer.querySelectorAll('.btn-item-action').forEach(btn => {
            btn.addEventListener('click', async (e) => {
                const action = btn.getAttribute('data-action');
                const id = btn.getAttribute('data-id');
                const item = downloadQueue.find(i => i.id === id);
                if (!item) return;

                if (action === 'delete') {
                    downloadQueue = downloadQueue.filter(i => i.id !== id);
                    saveQueue();
                    renderQueue();
                    showToast('Item removed from queue', 'info');
                    processQueueNext();
                } else if (action === 'cancel') {
                    try {
                        await invokeIPC('cancel_download', { id: item.id });
                        item.status = 'Cancelled';
                        saveQueue();
                        renderQueue();
                        showToast('Cancelling download...', 'info');
                        processQueueNext();
                    } catch (err) {
                        showToast('Could not cancel download', 'error');
                    }
                } else if (action === 'retry') {
                    item.status = 'Queued';
                    item.percent = 0;
                    item.errorDetails = null;
                    saveQueue();
                    renderQueue();
                    processQueueNext();
                } else if (action === 'open') {
                    if (item.filePath) {
                        invokeIPC('open_folder', { path: item.filePath }).catch(() => {
                            showToast('Could not open folder', 'error');
                        });
                    }
                } else if (action === 'details') {
                    showDiagnosticModal(item.errorDetails || 'No additional technical details.');
                }
            });
        });
    }

    async function startSingleDownload(item) {
        item.status = 'Downloading (1%)';
        item.percent = 1;
        item.errorDetails = null;
        saveQueue();
        renderQueue();

        // Non-blocking background metadata resolution if title is raw URL
        if (item.title === item.url) {
            invokeIPC('get_metadata', { url: item.url, language: item.category })
                .then(meta => {
                    item.title = meta.title;
                    item.thumb = meta.thumbnail;
                    if (item.category === 'Auto') {
                        item.category = meta.language;
                    }
                    saveQueue();
                    renderQueue();
                })
                .catch(() => {});
        }

        try {
            await invokeIPC('start_download', {
                request: {
                    id: item.id,
                    url: item.url,
                    mode: item.mode,
                    format: item.format,
                    quality: 'best',
                    language: item.category,
                    title: item.title,
                    cookies_from_browser: appSettings ? appSettings.cookies_from_browser : 'none',
                    cookies_file_path: appSettings ? appSettings.cookies_file_path : null,
                    download_subtitles: appSettings ? appSettings.download_subtitles : false,
                    subtitle_languages: appSettings ? appSettings.subtitle_languages : 'en',
                    duplicate_action: appSettings ? appSettings.duplicate_action : 'skip'
                }
            });
            showToast(`Download active: ${item.title}`, 'info');
        } catch (err) {
            item.status = 'Error: ' + (err || 'Launch failed');
            item.errorDetails = String(err);
            saveQueue();
            renderQueue();
            showToast(`Task error: ${err}`, 'error');
            processQueueNext();
        }
    }

    // 3. Process All Queued Items
    startAllBtn.addEventListener('click', async () => {
        const queuedItems = downloadQueue.filter(i => i.status === 'Queued' || i.status.startsWith('Error') || i.status.includes('Cancelled'));
        if (queuedItems.length === 0) {
            showToast('No pending items in queue to process', 'info');
            return;
        }

        queuedItems.forEach(i => {
            i.status = 'Queued';
            i.percent = 0;
            i.errorDetails = null;
        });
        saveQueue();
        renderQueue();

        showToast(`Processing ${queuedItems.length} pending item(s)...`, 'info');
        processQueueNext();
    });

    clearCompletedBtn.addEventListener('click', () => {
        const prevCount = downloadQueue.length;
        downloadQueue = downloadQueue.filter(i => !i.status.includes('Completed'));
        if (downloadQueue.length < prevCount) {
            saveQueue();
            renderQueue();
            showToast('Cleared completed items from queue', 'info');
        }
    });

    // 4. Media Library Tab Loading & Filters
    async function loadLibrary() {
        if (!musicLibraryList || !videoLibraryList) return;

        const search = libSearchInput ? libSearchInput.value.trim() : '';
        const sortBy = libSortSelect ? libSortSelect.value : 'name';
        const filterLang = libLangFilter ? libLangFilter.value : 'All';

        try {
            const data = await invokeIPC('list_library', {
                search: search || null,
                sortBy: sortBy || null,
                filterLang: filterLang || null,
                filterType: 'All'
            });

            renderLibrarySection(musicLibraryList, data.music || {}, 'audio');
            renderLibrarySection(videoLibraryList, data.videos || {}, 'video');
        } catch (err) {
            console.log('Library fetch error:', err);
        }
    }

    // Desktop Notifications Helper
    if ('Notification' in window && Notification.permission === 'default') {
        Notification.requestPermission().catch(() => {});
    }

    function sendDesktopNotification(title, body) {
        if ('Notification' in window && Notification.permission === 'granted') {
            try { new Notification(title, { body }); } catch (e) {}
        }
    }

    // Clipboard Media Link Auto-Detection
    let lastClipboardUrl = '';
    window.addEventListener('focus', async () => {
        try {
            if (!navigator.clipboard || !navigator.clipboard.readText) return;
            const text = await navigator.clipboard.readText();
            const trimmed = (text || '').trim();
            if (trimmed && (trimmed.includes('youtube.com') || trimmed.includes('youtu.be') || trimmed.includes('music.youtube.com')) && trimmed !== lastClipboardUrl) {
                lastClipboardUrl = trimmed;
                showClipboardToast(trimmed);
            }
        } catch (e) {}
    });

    function showClipboardToast(url) {
        const container = document.getElementById('toastContainer');
        if (!container) return;
        const toast = document.createElement('div');
        toast.className = 'toast info';
        toast.innerHTML = `
            <i class="fa-solid fa-link"></i>
            <div style="flex: 1;">
                <div><strong>Media Link Detected!</strong></div>
                <div style="font-size:0.75rem; color:#9ca3af; white-space:nowrap; overflow:hidden; text-overflow:ellipsis; max-width:200px;">${escapeHtml(url)}</div>
            </div>
            <button class="btn btn-sm btn-primary" id="clipAddBtn"><i class="fa-solid fa-plus"></i> Add</button>
        `;
        container.appendChild(toast);
        const btn = toast.querySelector('#clipAddBtn');
        if (btn) {
            btn.addEventListener('click', () => {
                if (urlInput) urlInput.value = url;
                toast.remove();
                showToast('Pasted media URL into input.', 'success');
            });
        }
        setTimeout(() => { if (toast.parentNode) toast.remove(); }, 6000);
    }

    // Playlist Extractor Controller
    if (parsePlaylistBtn && playlistModal) {
        parsePlaylistBtn.addEventListener('click', async () => {
            const url = urlInput ? urlInput.value.trim() : '';
            if (!url) {
                showToast('Please enter a YouTube playlist or video URL first', 'error');
                return;
            }

            parsePlaylistBtn.disabled = true;
            parsePlaylistBtn.innerHTML = '<i class="fa-solid fa-spinner fa-spin"></i> Parsing...';
            showToast('Extracting playlist tracks...', 'info');

            try {
                const tracks = await invokeIPC('get_playlist_info', { url });
                parsedPlaylistTracks = tracks || [];
                renderPlaylistModal();
                playlistModal.classList.remove('hidden');
                showToast(`Parsed ${parsedPlaylistTracks.length} track(s) from playlist`, 'success');
            } catch (err) {
                showToast(`Failed to parse playlist: ${err}`, 'error');
            } finally {
                parsePlaylistBtn.disabled = false;
                parsePlaylistBtn.innerHTML = '<i class="fa-solid fa-list-ol"></i> Parse Playlist';
            }
        });
    }

    if (closePlaylistModalBtn && playlistModal) {
        closePlaylistModalBtn.addEventListener('click', () => {
            playlistModal.classList.add('hidden');
        });
    }

    function renderPlaylistModal() {
        if (!playlistTracksList) return;
        playlistTracksList.innerHTML = '';

        if (parsedPlaylistTracks.length === 0) {
            playlistTracksList.innerHTML = '<div class="empty-queue">No tracks found in playlist</div>';
            return;
        }

        parsedPlaylistTracks.forEach((t, idx) => {
            const row = document.createElement('div');
            row.className = 'queue-item';
            row.style.padding = '0.5rem 0.85rem';
            row.innerHTML = `
                <input type="checkbox" class="playlist-track-cb" data-idx="${idx}" checked style="width:18px; height:18px; accent-color: var(--primary); cursor:pointer;">
                <img src="${t.thumbnail || 'https://via.placeholder.com/40'}" style="width:40px; height:40px; border-radius:4px; object-fit:cover;">
                <div style="flex:1; overflow:hidden;">
                    <div style="font-size:0.85rem; font-weight:600; white-space:nowrap; overflow:hidden; text-overflow:ellipsis;" title="${escapeHtml(t.title)}">${escapeHtml(t.title)}</div>
                    <div style="font-size:0.75rem; color:var(--text-muted);">${formatTime(t.duration)}</div>
                </div>
            `;
            playlistTracksList.appendChild(row);
        });

        updateSelectedPlaylistCount();

        playlistTracksList.querySelectorAll('.playlist-track-cb').forEach(cb => {
            cb.addEventListener('change', updateSelectedPlaylistCount);
        });
    }

    if (playlistSelectAll) {
        playlistSelectAll.addEventListener('change', () => {
            const checked = playlistSelectAll.checked;
            document.querySelectorAll('.playlist-track-cb').forEach(cb => {
                cb.checked = checked;
            });
            updateSelectedPlaylistCount();
        });
    }

    function updateSelectedPlaylistCount() {
        const checkedCount = document.querySelectorAll('.playlist-track-cb:checked').length;
        if (selectedTrackCount) selectedTrackCount.textContent = checkedCount;
    }

    if (queueSelectedPlaylistBtn) {
        queueSelectedPlaylistBtn.addEventListener('click', () => {
            const selectedCbs = document.querySelectorAll('.playlist-track-cb:checked');
            if (selectedCbs.length === 0) {
                showToast('Please select at least one track to queue', 'error');
                return;
            }

            const masterCat = playlistMasterCategory ? playlistMasterCategory.value : 'Auto';
            let addedCount = 0;

            selectedCbs.forEach(cb => {
                const idx = parseInt(cb.getAttribute('data-idx'), 10);
                const track = parsedPlaylistTracks[idx];
                if (track) {
                    const queueId = 'dl_' + Date.now() + '_' + Math.random().toString(36).substr(2, 4);
                    downloadQueue.push({
                        id: queueId,
                        url: track.url,
                        title: track.title,
                        thumb: track.thumbnail,
                        mode: modeSelect.value,
                        format: formatSelect.value,
                        category: masterCat,
                        status: 'Queued',
                        percent: 0,
                        speed: '',
                        eta: '',
                        filePath: null,
                        errorDetails: null
                    });
                    addedCount++;
                }
            });

            saveQueue();
            renderQueue();
            playlistModal.classList.add('hidden');
            showToast(`Queued ${addedCount} playlist track(s).`, 'success');
            processQueueNext();
        });
    }

    // Tag Editor Modal Controller
    if (closeTagModalBtn && tagEditorModal) {
        closeTagModalBtn.addEventListener('click', () => {
            tagEditorModal.classList.add('hidden');
        });
    }

    if (fetchLyricsBtn) {
        fetchLyricsBtn.addEventListener('click', async () => {
            const title = editTitle.value.trim();
            const artist = editArtist.value.trim();
            const fpath = editFilePath.value.trim();

            if (!title || !artist) {
                showToast('Please enter Title and Artist first', 'error');
                return;
            }

            fetchLyricsBtn.disabled = true;
            fetchLyricsBtn.innerHTML = '<i class="fa-solid fa-spinner fa-spin"></i> Fetching...';
            showToast(`Searching lyrics for "${title}" by ${artist}...`, 'info');

            try {
                const lyrics = await invokeIPC('fetch_lyrics', { artist, title, filePath: fpath || null });
                if (editLyrics) editLyrics.value = lyrics;
                showToast('Lyrics retrieved and embedded!', 'success');
            } catch (err) {
                showToast(`Lyrics lookup error: ${err}`, 'error');
            } finally {
                fetchLyricsBtn.disabled = false;
                fetchLyricsBtn.innerHTML = '<i class="fa-solid fa-align-left"></i> Fetch Lyrics (LRCLIB)';
            }
        });
    }

    if (tagEditorForm) {
        tagEditorForm.addEventListener('submit', async (e) => {
            e.preventDefault();
            const path = editFilePath.value;
            const title = editTitle.value.trim();
            const artist = editArtist.value.trim();
            const category = editCategory.value;
            const coverPath = editCoverPath ? editCoverPath.value.trim() : null;

            try {
                const newPath = await invokeIPC('edit_file_tags', {
                    path,
                    title,
                    artist,
                    category,
                    coverPath: coverPath || null
                });

                if (editLyrics && editLyrics.value.trim()) {
                    await invokeIPC('fetch_lyrics', { artist, title, filePath: newPath }).catch(() => {});
                }

                tagEditorModal.classList.add('hidden');
                showToast(`Updated tags for "${title}"`, 'success');
                loadLibrary();
            } catch (err) {
                showToast(`Failed to update tags: ${err}`, 'error');
            }
        });
    }

    function renderLibrarySection(container, langDict, mediaType) {
        container.innerHTML = '';
        if (Object.keys(langDict).length === 0) {
            container.innerHTML = '<div class="empty-queue">No matching files found in library</div>';
            return;
        }

        for (const [lang, files] of Object.entries(langDict)) {
            const group = document.createElement('div');
            group.className = 'lang-group';

            const header = document.createElement('div');
            header.className = 'lang-header';
            header.innerHTML = `
                <span><i class="fa-solid fa-folder-closed"></i> ${escapeHtml(lang)} (${files.length})</span>
                <i class="fa-solid fa-chevron-down"></i>
            `;

            const fileList = document.createElement('div');
            fileList.className = 'file-list';

            files.forEach(fileObj => {
                const fname = typeof fileObj === 'string' ? fileObj : fileObj.name;
                const fpath = typeof fileObj === 'string' ? '' : fileObj.path;
                const fsize = typeof fileObj === 'string' ? '' : formatBytes(fileObj.size);

                const item = document.createElement('div');
                item.className = 'file-item';
                item.innerHTML = `
                    <span class="file-name" title="${escapeHtml(fname)}"><i class="${mediaType === 'audio' ? 'fa-solid fa-music' : 'fa-solid fa-film'}"></i> ${escapeHtml(fname)} ${fsize ? `<small>(${fsize})</small>` : ''}</span>
                    <div class="file-item-actions">
                        ${fpath ? `<button class="btn-item-action edit-btn" data-path="${escapeHtml(fpath)}" data-name="${escapeHtml(fname)}" data-lang="${escapeHtml(lang)}" title="Edit Tags & Metadata"><i class="fa-solid fa-pen"></i></button>` : ''}
                        ${fpath ? `<button class="btn-item-action play-btn" data-path="${escapeHtml(fpath)}" title="Play Media"><i class="fa-solid fa-play"></i></button>` : ''}
                        ${fpath ? `<button class="btn-item-action open-btn" data-path="${escapeHtml(fpath)}" title="Open Folder"><i class="fa-solid fa-folder-open"></i></button>` : ''}
                    </div>
                `;
                fileList.appendChild(item);
            });

            group.appendChild(header);
            group.appendChild(fileList);
            container.appendChild(group);

            header.addEventListener('click', () => {
                fileList.style.display = fileList.style.display === 'none' ? 'flex' : 'none';
            });
        }

        // Action event listeners for library files
        container.querySelectorAll('.btn-item-action').forEach(btn => {
            btn.addEventListener('click', (e) => {
                e.stopPropagation();
                const path = btn.getAttribute('data-path');
                if (!path) return;

                if (btn.classList.contains('play-btn')) {
                    invokeIPC('open_file', { path }).catch(() => showToast('Could not play file', 'error'));
                } else if (btn.classList.contains('open-btn')) {
                    invokeIPC('open_folder', { path }).catch(() => showToast('Could not open folder', 'error'));
                } else if (btn.classList.contains('edit-btn')) {
                    const name = btn.getAttribute('data-name') || '';
                    const lang = btn.getAttribute('data-lang') || 'General';
                    const titleNoExt = name.substring(0, name.lastIndexOf('.')) || name;
                    
                    if (editFilePath) editFilePath.value = path;
                    if (editTitle) editTitle.value = titleNoExt;
                    if (editArtist) editArtist.value = 'Unknown Artist';
                    if (editCategory) editCategory.value = lang;
                    if (editCoverPath) editCoverPath.value = '';
                    if (editLyrics) editLyrics.value = '';
                    if (tagEditorModal) tagEditorModal.classList.remove('hidden');
                }
            });
        });
    }

    if (refreshLibBtn) refreshLibBtn.addEventListener('click', loadLibrary);
    if (libSearchInput) libSearchInput.addEventListener('input', debounce(loadLibrary, 300));
    if (libSortSelect) libSortSelect.addEventListener('change', loadLibrary);
    if (libLangFilter) libLangFilter.addEventListener('change', loadLibrary);

    // Initial Load
    await loadSystemInfo();
    await loadSettings();
    loadQueue();
    loadLibrary();

    // Helpers
    function formatTime(seconds) {
        if (!seconds) return '00:00';
        const m = Math.floor(seconds / 60);
        const s = Math.floor(seconds % 60);
        return `${m}:${s < 10 ? '0' : ''}${s}`;
    }

    function formatBytes(bytes) {
        if (!bytes) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    }

    function escapeHtml(str) {
        return (str || '').replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }

    function debounce(func, wait) {
        let timeout;
        return function(...args) {
            clearTimeout(timeout);
            timeout = setTimeout(() => func.apply(this, args), wait);
        };
    }
});
