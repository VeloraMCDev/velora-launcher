export function composite(foreground, background) {
  return foreground.slice(0, 3).map((channel, index) => channel * foreground[3] + background[index] * (1 - foreground[3])).concat(1);
}

export function contrastRatio(first, second) {
  const luminance = color => color.slice(0, 3).map(channel => channel / 255)
    .map(channel => channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4)
    .reduce((total, channel, index) => total + channel * [0.2126, 0.7152, 0.0722][index], 0);
  const values = [luminance(first), luminance(second)];
  return (Math.max(...values) + 0.05) / (Math.min(...values) + 0.05);
}

export function textThreshold(fontSize, fontWeight) {
  return fontSize >= 24 || (fontSize >= 56 / 3 && fontWeight >= 700) ? 3 : 4.5;
}

export function auditRenderedText(root = document.body) {
  const document = root.ownerDocument;
  const window = document.defaultView;
  const context = document.createElement('canvas').getContext('2d', { willReadFrequently: true });
  if (!context || !window) throw new Error('Rendered audit requires a browser document and canvas');
  const rgba = value => {
    context.clearRect(0, 0, 1, 1);
    context.fillStyle = '#ff00ff';
    context.fillStyle = value;
    context.fillRect(0, 0, 1, 1);
    return [...context.getImageData(0, 0, 1, 1).data].map((channel, index) => index === 3 ? channel / 255 : channel);
  };
  const result = { checked: 0, failures: [], manual: [] };
  for (const element of [root, ...root.querySelectorAll('*')]) {
    const text = [...element.childNodes].filter(node => node.nodeType === 3).map(node => node.textContent.trim()).filter(Boolean).join(' ');
    if (!text || element.closest('svg, script, style, [inert], [aria-hidden="true"], [disabled]')) continue;
    if (!element.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) continue;
    const style = window.getComputedStyle(element);
    const entry = { text: text.slice(0, 100), element: element.tagName.toLowerCase(), className: element.getAttribute('class') ?? '' };
    const ancestors = [];
    for (let ancestor = element; ancestor; ancestor = ancestor.parentElement) ancestors.unshift(ancestor);
    const complex = ancestors.some(ancestor => {
      const computed = window.getComputedStyle(ancestor);
      return computed.backgroundImage !== 'none' || Number(computed.opacity) !== 1 || computed.mixBlendMode !== 'normal' || computed.filter !== 'none' || computed.backdropFilter !== 'none';
    });
    if (complex || style.textShadow !== 'none' || style.webkitTextFillColor !== style.color) {
      result.manual.push(entry);
      continue;
    }
    let background = [255, 255, 255, 1];
    for (const ancestor of ancestors) background = composite(rgba(window.getComputedStyle(ancestor).backgroundColor), background);
    const foreground = composite(rgba(style.color), background);
    const ratio = contrastRatio(foreground, background);
    const threshold = textThreshold(parseFloat(style.fontSize), Number(style.fontWeight));
    result.checked++;
    if (ratio < threshold) result.failures.push({ ...entry, ratio, threshold, foreground, background });
  }
  return result;
}
