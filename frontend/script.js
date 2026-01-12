document.addEventListener('DOMContentLoaded', () => {
    const codeEditor = document.getElementById('codeEditor');
    const runBtn = document.getElementById('runBtn');
    const clearBtn = document.getElementById('clearBtn');
    const output = document.getElementById('output');

    codeEditor.addEventListener('keydown', (e) => {
        if (e.key === 'Tab') {
            e.preventDefault();
            const start = codeEditor.selectionStart;
            const end = codeEditor.selectionEnd;
            codeEditor.value = codeEditor.value.substring(0, start) + '    ' + codeEditor.value.substring(end);
            codeEditor.selectionStart = codeEditor.selectionEnd = start + 4;
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
