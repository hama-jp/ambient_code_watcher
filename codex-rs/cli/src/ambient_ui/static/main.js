// UI文字列の定数
const UI_STRINGS = {
    CONNECTING: '接続中...',
    CONNECTED: '接続済み',
    DISCONNECTED: '切断済み',
    CONNECTION_FAILED: '接続失敗',
    WELCOME_MESSAGE: 'Ambient Watcherに接続しました',
    CONNECTION_LOST: 'サーバーとの接続が失われました。3秒後に再接続を試みます...',
    CONNECTION_RESTORED: 'サーバーに再接続しました',
    CONNECTION_CLOSED_CLEAN: 'サーバーとの接続が正常に終了しました',
    CONNECTION_ERROR: 'サーバーとの通信エラーが発生しました',
    CONNECTION_FINAL_ERROR: 'サーバーへの接続に失敗しました。ページを再読み込みしてください。',
    NOT_CONNECTED: 'サーバーに接続されていません。接続を待っています...',
    PARSE_ERROR: 'サーバーからの不正なメッセージを受信しました',
    LAST_UPDATE: '最終更新'
};

// CSSクラス名の定数
const CSS_CLASSES = {
    CONNECTED: 'connected',
    DISCONNECTED: 'disconnected',
    ERROR: 'error',
    LOG_ENTRY: 'log-entry',
    SYSTEM_MESSAGE: 'system-message',
    USER_QUERY: 'user-query',
    QUERY_RESPONSE: 'query-response',
    ANALYSIS: 'analysis',
    SYSTEM: 'system',
    INFO: 'info',
    WARNING: 'warning',
    SUCCESS: 'success'
};

// 設定値の定数
const CONFIG = {
    MAX_RECONNECT_ATTEMPTS: 5,
    RECONNECT_DELAY_MS: 3000,
    SCROLL_DELAY_MS: 100
};

document.addEventListener('DOMContentLoaded', () => {
    const logContainer = document.getElementById('log-container');
    const queryForm = document.getElementById('query-form');
    const queryInput = document.getElementById('query-input');
    const statusDiv = document.getElementById('status');
    const lastUpdateDiv = document.getElementById('last-update');

    let socket;
    let reconnectTimeout = null;
    let reconnectAttempts = 0;
    let queryCounter = 0; // 質問のカウンター
    let currentQueryId = null; // 現在処理中の質問ID
    
    // エラーメッセージをUIに表示する関数
    function showMessage(message, type = CSS_CLASSES.INFO) {
        const logEntry = document.createElement('div');
        logEntry.classList.add(CSS_CLASSES.LOG_ENTRY, CSS_CLASSES.SYSTEM_MESSAGE, type);
        const timestamp = new Date().toLocaleTimeString('ja-JP');
        const safeMessage = typeof DOMPurify !== 'undefined'
            ? DOMPurify.sanitize(message, {ALLOWED_TAGS: []})
            : message.replace(/</g, '&lt;').replace(/>/g, '&gt;');
        logEntry.innerHTML = `<span class="timestamp">[${timestamp}]</span> ${safeMessage}`;
        logContainer.appendChild(logEntry);
        logContainer.scrollTop = logContainer.scrollHeight;
    }
    
    function updateLastTime() {
        const now = new Date();
        const timeStr = now.toLocaleTimeString('ja-JP', { 
            hour: '2-digit', 
            minute: '2-digit', 
            second: '2-digit' 
        });
        lastUpdateDiv.textContent = `${UI_STRINGS.LAST_UPDATE}: ${timeStr}`;
    }

    function connect() {
        // 既存の接続とタイムアウトをクリーンアップ
        if (socket) {
            socket.onopen = null;
            socket.onmessage = null;
            socket.onclose = null;
            socket.onerror = null;
            if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
                socket.close();
            }
        }
        
        if (reconnectTimeout) {
            clearTimeout(reconnectTimeout);
            reconnectTimeout = null;
        }
        
        // Use the current host and port for the WebSocket connection.
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const host = window.location.host;
        socket = new WebSocket(`${protocol}//${host}/ws`);

        socket.onopen = () => {
            statusDiv.textContent = UI_STRINGS.CONNECTED;
            statusDiv.className = CSS_CLASSES.CONNECTED;
            reconnectAttempts = 0; // リセット
            if (reconnectAttempts > 0) {
                showMessage(UI_STRINGS.CONNECTION_RESTORED, CSS_CLASSES.SUCCESS);
            }
        };

        socket.onmessage = (event) => {
            let data;
            try {
                data = JSON.parse(event.data);
            } catch (e) {
                console.error('Failed to parse WebSocket message:', e);
                console.error('Raw message:', event.data);
                showMessage(UI_STRINGS.PARSE_ERROR, CSS_CLASSES.ERROR);
                return;
            }
            
            const logEntry = document.createElement('div');
            logEntry.classList.add('log-entry');

            if (data.System) {
                logEntry.classList.add(CSS_CLASSES.SYSTEM);
                logEntry.textContent = data.System;
            } else if (data.Analysis) {
                logEntry.classList.add(CSS_CLASSES.ANALYSIS);
                // 分析データが来たら最終更新時間を更新
                updateLastTime();
                
                // Markdownをレンダリング
                const isMarkdown = data.Analysis.includes('##') || 
                                 data.Analysis.includes('**') || 
                                 data.Analysis.includes('```') ||
                                 data.Analysis.includes('|') ||
                                 data.Analysis.includes('- ');
                
                if (isMarkdown && typeof marked !== 'undefined' && typeof DOMPurify !== 'undefined') {
                    const rawHtml = marked.parse(data.Analysis);
                    logEntry.innerHTML = DOMPurify.sanitize(rawHtml);
                } else {
                    logEntry.textContent = data.Analysis;
                }
            } else if (data.UserQuery) {
                // 新しい質問が来たら、カウンターを増やしてIDを設定
                queryCounter++;
                currentQueryId = queryCounter;
                logEntry.classList.add(CSS_CLASSES.USER_QUERY);
                logEntry.setAttribute('data-query-id', currentQueryId);
                const safeQuery = typeof DOMPurify !== 'undefined' 
                    ? DOMPurify.sanitize(data.UserQuery, {ALLOWED_TAGS: []}) 
                    : data.UserQuery.replace(/</g, '&lt;').replace(/>/g, '&gt;');
                logEntry.innerHTML = `<span class="query-badge">Q${currentQueryId}</span> You: ${safeQuery}`;
            } else if (data.QueryResponse) {
                // 質問への回答
                logEntry.classList.add(CSS_CLASSES.ANALYSIS, CSS_CLASSES.QUERY_RESPONSE);
                if (currentQueryId) {
                    logEntry.setAttribute('data-query-id', currentQueryId);
                    const content = (marked && typeof DOMPurify !== 'undefined') 
                        ? DOMPurify.sanitize(marked.parse(data.QueryResponse)) 
                        : data.QueryResponse;
                    logEntry.innerHTML = DOMPurify.sanitize(`<span class="query-badge">A${currentQueryId}</span> ${content}`);
                } else {
                    logEntry.innerHTML = (marked && typeof DOMPurify !== 'undefined')
                        ? DOMPurify.sanitize(marked.parse(data.QueryResponse))
                        : data.QueryResponse;
                }
                updateLastTime();
            }

            logContainer.appendChild(logEntry);
            logContainer.scrollTop = logContainer.scrollHeight;
        };

        socket.onclose = (event) => {
            statusDiv.textContent = UI_STRINGS.DISCONNECTED;
            statusDiv.className = CSS_CLASSES.DISCONNECTED;
            
            if (event.wasClean) {
                showMessage(UI_STRINGS.CONNECTION_CLOSED_CLEAN, CSS_CLASSES.INFO);
            } else {
                reconnectAttempts++;
                if (reconnectAttempts <= CONFIG.MAX_RECONNECT_ATTEMPTS) {
                    showMessage(`${UI_STRINGS.CONNECTION_LOST} (${reconnectAttempts}/${CONFIG.MAX_RECONNECT_ATTEMPTS})`, CSS_CLASSES.WARNING);
                    // 再接続を試みる
                    reconnectTimeout = setTimeout(connect, CONFIG.RECONNECT_DELAY_MS);
                } else {
                    showMessage(UI_STRINGS.CONNECTION_FINAL_ERROR, CSS_CLASSES.ERROR);
                    statusDiv.textContent = UI_STRINGS.CONNECTION_FAILED;
                    statusDiv.className = CSS_CLASSES.ERROR;
                }
            }
        };

        socket.onerror = (error) => {
            console.error('WebSocket Error:', error);
            showMessage(UI_STRINGS.CONNECTION_ERROR, CSS_CLASSES.ERROR);
            if (socket.readyState === WebSocket.OPEN) {
                socket.close();
            }
        };
    }

    queryForm.addEventListener('submit', (e) => {
        e.preventDefault();
        const query = queryInput.value.trim();
        
        if (!query) {
            return;
        }
        
        if (!socket || socket.readyState !== WebSocket.OPEN) {
            showMessage(UI_STRINGS.NOT_CONNECTED, CSS_CLASSES.WARNING);
            return;
        }
        
        if (query && socket && socket.readyState === WebSocket.OPEN) {
            socket.send(query);
            queryInput.value = '';
            // サーバーから UserQuery イベントが返ってくるので、ここでは表示しない
            
            // スクロールを最下部に
            setTimeout(() => {
                logContainer.scrollTop = logContainer.scrollHeight;
            }, CONFIG.SCROLL_DELAY_MS);
        }
    });

    // ページ離脱時のクリーンアップ
    window.addEventListener('beforeunload', () => {
        if (reconnectTimeout) {
            clearTimeout(reconnectTimeout);
        }
        if (socket && socket.readyState === WebSocket.OPEN) {
            socket.close();
        }
    });
    
    connect();
});
