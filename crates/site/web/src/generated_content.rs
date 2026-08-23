//! Shared browser resources for build-time generated content.

pub const KATEX_STYLESHEET_URL: &str =
    "https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.css";
pub const KATEX_STYLESHEET_INTEGRITY: &str =
    "sha384-5TcZemv2l/9On385z///+d7MSYlvIEw9FuZTIdZ14vJLqWphw7e7ZPuOiCHJcFCP";
pub const KATEX_SCRIPT_URL: &str = "https://cdn.jsdelivr.net/npm/katex@0.16.22/dist/katex.min.js";
pub const KATEX_SCRIPT_INTEGRITY: &str =
    "sha384-cMkvdD8LoxVzGF/RPUKAcvmm49FQ0oxwDF3BGKtDXcEc+T1b2N+teh/OJfpU0jr6";
pub const HIGHLIGHT_STYLESHEET_URL: &str =
    "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/styles/github-dark.min.css";
pub const HIGHLIGHT_SCRIPT_URL: &str =
    "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/highlight.min.js";

pub const MATH_RENDER_SCRIPT: &str = r#"
window.okawakRenderMath = function(root) {
if (!window.katex) return;

const scope = root || document.body;
const normalizeExpression = (value) =>
(value || '').replace(/[\u2009\u200A\u200B\u200C\u200D\u2061\u202F\u2060\uFEFF]/g, '');

scope.querySelectorAll('.math-inline').forEach((element) => {
if (element.dataset.katexRendered === 'true') return;

const expression = normalizeExpression(element.textContent);
window.katex.render(expression, element, {
displayMode: false,
throwOnError: false,
});
element.dataset.katexRendered = 'true';
});

scope.querySelectorAll('.math-display').forEach((element) => {
if (element.dataset.katexRendered === 'true') return;

const expression = normalizeExpression(element.textContent);
window.katex.render(expression, element, {
displayMode: true,
throwOnError: false,
});
element.dataset.katexRendered = 'true';
});
};

window.okawakScheduleMathRender = function(root) {
let remaining = 200;
const attempt = function() {
if (window.katex && window.okawakRenderMath) {
window.okawakRenderMath(root);
return;
}

if (remaining > 0) {
remaining -= 1;
window.setTimeout(attempt, 50);
}
};

attempt();
};
"#;

pub const CODE_HIGHLIGHT_SCRIPT: &str = r#"
window.okawakHighlightCode = function(root) {
if (!window.hljs) return;
const scope = root || document.body;
scope.querySelectorAll('.content-prose pre code:not([data-highlighted])')
.forEach((element) => window.hljs.highlightElement(element));
};
window.okawakScheduleCodeHighlight = function(root) {
let remaining = 200;
const attempt = function() {
if (window.hljs && window.okawakHighlightCode) {
window.okawakHighlightCode(root);
return;
}
if (remaining > 0) {
remaining -= 1;
window.setTimeout(attempt, 50);
}
};
attempt();
};
"#;
