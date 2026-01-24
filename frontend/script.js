document.addEventListener('DOMContentLoaded', () => {
    const codeEditor = document.getElementById('codeEditor');
    const highlighted = document.getElementById('highlighted').querySelector('code');
    const runBtn = document.getElementById('runBtn');
    const clearBtn = document.getElementById('clearBtn');
    const output = document.getElementById('output');
    const profiler = document.getElementById('profiler');
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

        const macros = ['println', 'print', 'format', 'vec', 'panic', 'assert', 'assert_eq', 'assert_ne', 'debug_assert', 'debug_assert_eq', 'todo', 'unimplemented', 'unreachable', 'include', 'include_str', 'include_bytes', 'env', 'concat', 'stringify', 'cfg', 'line', 'column', 'file', 'module_path'];
        const macroPattern = new RegExp(`\\b(${macros.join('|')})!`, 'g');
        escaped = escaped.replace(macroPattern, (m) => storeToken(m, 'hl-macro'));

        escaped = escaped.replace(/\b([a-z_][a-z0-9_]*)\s*\(/gi, (match, name) => {
            if (!keywords.includes(name) && !types.includes(name)) {
                return `<span class="hl-function">${name}</span>(`;
            }
            return match;
        });

        tokens.forEach(({ placeholder, html }) => {
            escaped = escaped.replace(placeholder, html);
        });

        return escaped;
    }

    const lineNumbers = document.getElementById('lineNumbers');

    function updateLineNumbers() {
        const lines = codeEditor.value.split('\n').length;
        let html = '';
        for (let i = 1; i <= lines; i++) {
            html += `<span>${i}</span>`;
        }
        lineNumbers.innerHTML = html;
    }

    function updateHighlight() {
        highlighted.innerHTML = highlightRust(codeEditor.value) + '\n';
        updateLineNumbers();
    }

    function syncScroll() {
        const pre = document.getElementById('highlighted');
        pre.scrollTop = codeEditor.scrollTop;
        pre.scrollLeft = codeEditor.scrollLeft;
        lineNumbers.scrollTop = codeEditor.scrollTop;
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

        if (e.key === 'Enter' && !e.ctrlKey && !e.metaKey) {
            e.preventDefault();
            const start = codeEditor.selectionStart;
            const value = codeEditor.value;
            const lineStart = value.lastIndexOf('\n', start - 1) + 1;
            const currentLine = value.substring(lineStart, start);
            const indent = currentLine.match(/^(\s*)/)[1];
            const charBefore = value[start - 1];
            const charAfter = value[start];
            let newIndent = indent;
            if (charBefore === '{' || charBefore === '(' || charBefore === '[') {
                newIndent = indent + '    ';
            }
            if ((charBefore === '{' && charAfter === '}') || 
                (charBefore === '(' && charAfter === ')') || 
                (charBefore === '[' && charAfter === ']')) {
                codeEditor.value = value.substring(0, start) + '\n' + newIndent + '\n' + indent + value.substring(start);
                codeEditor.selectionStart = codeEditor.selectionEnd = start + 1 + newIndent.length;
            } else {
                codeEditor.value = value.substring(0, start) + '\n' + newIndent + value.substring(start);
                codeEditor.selectionStart = codeEditor.selectionEnd = start + 1 + newIndent.length;
            }
            updateHighlight();
        }

        if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
            e.preventDefault();
            runCode();
        }
    });

    const formatBtn = document.getElementById('formatBtn');

    runBtn.addEventListener('click', runCode);
    clearBtn.addEventListener('click', () => {
        output.innerHTML = '<span class="placeholder">Click "Run" to execute your code...</span>';
        output.className = 'output-content';
        profiler.innerHTML = '<span class="placeholder">Run your code to see timing data...</span>';
    });

    formatBtn.addEventListener('click', formatCode);

    function updateProfiler(result) {
        profiler.innerHTML = '';
        
        if (result.compile_time_ms !== undefined && result.compile_time_ms !== null) {
            const compileSec = (result.compile_time_ms / 1000).toFixed(2);
            const compileRow = document.createElement('div');
            compileRow.className = 'profiler-row compile-time';
            compileRow.innerHTML = `<span class="profiler-label">Compile Time</span><span class="profiler-value">${compileSec} s</span>`;
            profiler.appendChild(compileRow);
        }
        
        if (result.function_times && result.function_times.length > 0) {
            const funcHeader = document.createElement('div');
            funcHeader.className = 'profiler-section-header';
            funcHeader.textContent = 'Function Timings';
            profiler.appendChild(funcHeader);
            
            result.function_times.forEach(ft => {
                const funcRow = document.createElement('div');
                funcRow.className = 'profiler-row function-time';
                funcRow.innerHTML = `<span class="profiler-label">${ft.name}()</span><span class="profiler-value">${ft.time_ms.toFixed(3)} ms</span>`;
                profiler.appendChild(funcRow);
            });
        }
        
        if (result.run_time_ms !== undefined && result.run_time_ms !== null) {
            const runRow = document.createElement('div');
            runRow.className = 'profiler-row total';
            runRow.innerHTML = `<span class="profiler-label">Execution Time</span><span class="profiler-value">${result.run_time_ms} ms</span>`;
            profiler.appendChild(runRow);
        }
        
        if (profiler.children.length === 0) {
            profiler.innerHTML = '<span class="placeholder">No timing data available</span>';
        }
    }

    async function formatCode() {
        const code = codeEditor.value;
        if (!code.trim()) return;

        formatBtn.disabled = true;
        formatBtn.classList.add('loading');
        formatBtn.innerHTML = '<span class="btn-icon">&#8635;</span> Formatting...';

        try {
            const response = await fetch('/api/format', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ code }),
            });

            const result = await response.json();

            if (result.success) {
                codeEditor.value = result.formatted;
                updateHighlight();
            } else {
                output.textContent = 'Format error: ' + (result.error || 'Unknown error');
                output.className = 'output-content error';
            }
        } catch (err) {
            output.textContent = 'Format failed: ' + err.message;
            output.className = 'output-content error';
        } finally {
            formatBtn.disabled = false;
            formatBtn.classList.remove('loading');
            formatBtn.innerHTML = '<span class="btn-icon">&#8801;</span> Format';
        }
    }

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
            const cratesResponse = await fetch('/api/crates');
            const cratesData = await cratesResponse.json();
            const userCrates = cratesData.crates
                .filter(c => c.version !== 'builtin')
                .map(c => c.name);

            const response = await fetch('/api/run', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ code, crates: userCrates }),
            });

            const result = await response.json();
            
            updateProfiler(result);

            if (result.success) {
                let combined = '';
                if (result.error) combined += result.error;
                if (result.output) combined += (combined ? '\n' : '') + result.output;
                
                output.innerHTML = '';
                output.className = 'output-content success';
                
                if (combined) {
                    const textDiv = document.createElement('pre');
                    textDiv.textContent = combined;
                    output.appendChild(textDiv);
                }
                
                if (result.images && result.images.length > 0) {
                    const imagesDiv = document.createElement('div');
                    imagesDiv.className = 'output-images';
                    result.images.forEach(src => {
                        const img = document.createElement('img');
                        img.src = src;
                        img.className = 'output-image';
                        imagesDiv.appendChild(img);
                    });
                    output.appendChild(imagesDiv);
                }
                
                if (!combined && (!result.images || result.images.length === 0)) {
                    output.textContent = '(Program completed with no output)';
                }
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
