document.addEventListener('DOMContentLoaded', () => {
    const logContainer = document.getElementById('log-container');
    const queryForm = document.getElementById('query-form');
    const queryInput = document.getElementById('query-input');
    const statusDiv = document.getElementById('status');

    let socket;

    function connect() {
        // Use the current host and port for the WebSocket connection.
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const host = window.location.host;
        socket = new WebSocket(`${protocol}//${host}/ws`);

        socket.onopen = () => {
            statusDiv.textContent = 'Connected';
            statusDiv.className = 'connected';
        };

        socket.onmessage = (event) => {
            const data = JSON.parse(event.data);
            const logEntry = document.createElement('div');
            logEntry.classList.add('log-entry');

            if (data.System) {
                logEntry.classList.add('system');
                logEntry.textContent = data.System;
            } else if (data.Analysis) {
                logEntry.classList.add('analysis');
                logEntry.textContent = data.Analysis;
            } else if (data.UserQuery) {
                logEntry.classList.add('user-query');
                logEntry.textContent = `You: ${data.UserQuery}`;
            }

            logContainer.appendChild(logEntry);
            logContainer.scrollTop = logContainer.scrollHeight;
        };

        socket.onclose = () => {
            statusDiv.textContent = 'Disconnected';
            statusDiv.className = 'disconnected';
            // Try to reconnect after a delay
            setTimeout(connect, 3000);
        };

        socket.onerror = (error) => {
            console.error('WebSocket Error:', error);
            socket.close();
        };
    }

    queryForm.addEventListener('submit', (e) => {
        e.preventDefault();
        const query = queryInput.value.trim();
        if (query && socket && socket.readyState === WebSocket.OPEN) {
            socket.send(query);

            // Display the user's query in the log
            const logEntry = document.createElement('div');
            logEntry.classList.add('log-entry', 'user-query');
            logEntry.textContent = `You: ${query}`;
            logContainer.appendChild(logEntry);
            logContainer.scrollTop = logContainer.scrollHeight;

            queryInput.value = '';
        }
    });

    connect();
});
