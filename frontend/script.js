document.addEventListener('DOMContentLoaded', () => {
    const codeEditor = document.getElementById('codeEditor');
    const highlighted = document.getElementById('highlighted').querySelector('code');
    const runBtn = document.getElementById('runBtn');
    const clearBtn = document.getElementById('clearBtn');
    const output = document.getElementById('output');
    const statusBadge = document.getElementById('statusBadge');

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
        if (statusBadge) {
            statusBadge.textContent = isOnline ? 'Online' : 'Offline';
            statusBadge.className = `status-badge ${isOnline ? 'online' : 'offline'}`;
        }
    }

    checkStatus();
    setInterval(checkStatus, 30000);

    function highlightRust(code) {
        const tokens = [];
        let tokenId = 0;

        function storeToken(match, className) {
            const placeholder = `__TOKEN_${tokenId}__`;
            tokens.push({ placeholder, html: `<span class="${className}">${match}</span>` });
            tokenId++;
            return placeholder;
        }

        let escaped = code
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;');

        escaped = escaped.replace(/("(?:[^"\\]|\\.)*")/g, (m) => storeToken(m, 'hl-string'));

        escaped = escaped.replace(/(\/\/.*)/g, (m) => storeToken(m, 'hl-comment'));

        const keywords = ['fn', 'let', 'mut', 'const', 'if', 'else', 'match', 'for', 'while', 'loop', 'return', 'break', 'continue', 'struct', 'enum', 'impl', 'trait', 'pub', 'use', 'mod', 'crate', 'self', 'super', 'where', 'async', 'await', 'move', 'ref', 'static', 'type', 'unsafe', 'extern', 'dyn', 'as', 'in'];
        const keywordPattern = new RegExp(`\\b(${keywords.join('|')})\\b`, 'g');
        escaped = escaped.replace(keywordPattern, '<span class="hl-keyword">$1</span>');

        const types = ['i8', 'i16', 'i32', 'i64', 'i128', 'isize', 'u8', 'u16', 'u32', 'u64', 'u128', 'usize', 'f32', 'f64', 'bool', 'char', 'str', 'String', 'Vec', 'Option', 'Result', 'Box', 'Rc', 'Arc', 'Cell', 'RefCell', 'HashMap', 'HashSet', 'BTreeMap', 'BTreeSet'];
        const typePattern = new RegExp(`\\b(${types.join('|')})\\b`, 'g');
        escaped = escaped.replace(typePattern, '<span class="hl-type">$1</span>');

        escaped = escaped.replace(/\b(true|false|None|Some|Ok|Err)\b/g, '<span class="hl-literal">$1</span>');

        escaped = escaped.replace(/\b(\d+\.?\d*)\b/g, '<span class="hl-number">$1</span>');

        escaped = escaped.replace(/\b([a-z_][a-z0-9_]*)\s*!?\s*\(/gi, (match, name) => {
            if (!keywords.includes(name) && !types.includes(name)) {
                return `<span class="hl-function">${name}</span>(`;
            }
            return match;
        });

        escaped = escaped.replace(/\b(println|print|format|vec|panic|assert|assert_eq|assert_ne|debug_assert|debug_assert_eq|todo|unimplemented|unreachable)!/g, '<span class="hl-macro">$1!</span>');

        tokens.forEach(({ placeholder, html }) => {
            escaped = escaped.replace(placeholder, html);
        });

        return escaped;
    }

    function updateHighlight() {
        highlighted.innerHTML = highlightRust(codeEditor.value) + '\n';
    }

    function syncScroll() {
        const pre = document.getElementById('highlighted');
        pre.scrollTop = codeEditor.scrollTop;
        pre.scrollLeft = codeEditor.scrollLeft;
    }

    codeEditor.addEventListener('input', updateHighlight);
    codeEditor.addEventListener('scroll', syncScroll);

    updateHighlight();

    codeEditor.addEventListener('keydown', (e) => {
        if (e.key === 'Tab') {
            e.preventDefault();
            const start = codeEditor.selectionStart;
            const end = codeEditor.selectionEnd;
            codeEditor.value = codeEditor.value.substring(0, start) + '    ' + codeEditor.value.substring(end);
            codeEditor.selectionStart = codeEditor.selectionEnd = start + 4;
            updateHighlight();
        }

        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
            e.preventDefault();
            runCode();
        }
    });

    runBtn.addEventListener('click', runCode);
    clearBtn.addEventListener('click', () => {
        output.innerHTML = '<span class="placeholder">Click "Run" to execute your code...</span>';
        output.className = 'output-content';
    });

    async function runCode() {
        const code = codeEditor.value.trim();
        
        if (!code) {
            output.textContent = 'Please enter some Rust code to run.';
            output.className = 'output-content error';
            return;
        }

        runBtn.disabled = true;
        runBtn.classList.add('loading');
        runBtn.innerHTML = '<span class="btn-icon">&#8635;</span> Running...';
        output.textContent = 'Compiling and running...';
        output.className = 'output-content';

        try {
            const response = await fetch('/api/run', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ code }),
            });

            const result = await response.json();

            if (result.success) {
                output.textContent = result.output || '(Program completed with no output)';
                output.className = 'output-content success';
            } else {
                output.textContent = result.error || result.output || 'An error occurred';
                output.className = 'output-content error';
            }
        } catch (err) {
            output.textContent = `Network error: ${err.message}\n\nMake sure the server is running.`;
            output.className = 'output-content error';
        } finally {
            runBtn.disabled = false;
            runBtn.classList.remove('loading');
            runBtn.innerHTML = '<span class="btn-icon">&#9654;</span> Run';
        }
    }
});
