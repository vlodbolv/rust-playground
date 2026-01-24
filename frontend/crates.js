document.addEventListener('DOMContentLoaded', () => {
    const statusBadge = document.getElementById('statusBadge');
    const crateNameInput = document.getElementById('crateName');
    const crateVersionInput = document.getElementById('crateVersion');
    const addBtn = document.getElementById('addBtn');
    const addMessage = document.getElementById('addMessage');
    const cratesList = document.getElementById('cratesList');

    let isOnline = false;

    async function checkStatus() {
        try {
            const response = await fetch('/api/status');
            const data = await response.json();
            isOnline = data.online;
            updateStatusBadge();
        } catch (err) {
            isOnline = false;
            updateStatusBadge();
        }
    }

    function updateStatusBadge() {
        statusBadge.textContent = isOnline ? 'Online' : 'Offline';
        statusBadge.className = `status-badge ${isOnline ? 'online' : 'offline'}`;
        addBtn.disabled = !isOnline;
        
        if (!isOnline) {
            addBtn.title = 'Cannot add crates while offline';
        } else {
            addBtn.title = '';
        }
    }

    async function loadCrates() {
        try {
            const response = await fetch('/api/crates');
            const data = await response.json();
            renderCrates(data.crates);
        } catch (err) {
            cratesList.innerHTML = '<p class="placeholder">Failed to load crates</p>';
        }
    }

    function renderCrates(crates) {
        if (crates.length === 0) {
            cratesList.innerHTML = '<p class="placeholder">No crates installed yet</p>';
            return;
        }

        cratesList.innerHTML = crates.map(c => {
            const isBuiltin = c.version === 'builtin';
            return `
                <div class="crate-card ${isBuiltin ? 'builtin' : ''}" data-name="${c.name}">
                    <div class="crate-info">
                        <span class="crate-name">${c.name}</span>
                        <span class="crate-version">${isBuiltin ? 'built-in' : 'v' + c.version}</span>
                    </div>
                    ${isBuiltin ? '' : `<button class="remove-btn" onclick="removeCrate('${c.name}')">Remove</button>`}
                </div>
            `;
        }).join('');
    }

    window.removeCrate = async function(name) {
        try {
            const response = await fetch('/api/crates/remove', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name })
            });
            
            if (response.ok) {
                loadCrates();
            }
        } catch (err) {
            console.error('Failed to remove crate:', err);
        }
    };

    addBtn.addEventListener('click', async () => {
        const name = crateNameInput.value.trim();
        if (!name) {
            addMessage.textContent = 'Please enter a crate name';
            addMessage.className = 'message error';
            return;
        }

        addBtn.disabled = true;
        addBtn.textContent = 'Adding...';
        addMessage.textContent = '';

        try {
            const response = await fetch('/api/crates/add', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    name,
                    version: crateVersionInput.value.trim() || null
                })
            });

            const result = await response.json();

            if (result.success) {
                addMessage.textContent = `Added ${result.name} v${result.version}`;
                addMessage.className = 'message success';
                crateNameInput.value = '';
                crateVersionInput.value = '';
                loadCrates();
            } else {
                addMessage.textContent = result.error;
                addMessage.className = 'message error';
            }
        } catch (err) {
            addMessage.textContent = 'Failed to add crate';
            addMessage.className = 'message error';
        } finally {
            addBtn.disabled = !isOnline;
            addBtn.textContent = 'Add Crate';
        }
    });

    crateNameInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter' && isOnline) {
            addBtn.click();
        }
    });

    checkStatus();
    loadCrates();
    setInterval(checkStatus, 30000);

    const themeToggle = document.getElementById('themeToggle');
    
    function loadTheme() {
        const savedTheme = localStorage.getItem('theme');
        if (savedTheme === 'light') {
            document.body.classList.add('light-theme');
        }
    }

    function toggleTheme() {
        document.body.classList.toggle('light-theme');
        const isLight = document.body.classList.contains('light-theme');
        localStorage.setItem('theme', isLight ? 'light' : 'dark');
    }

    loadTheme();
    if (themeToggle) {
        themeToggle.addEventListener('click', toggleTheme);
    }
});
