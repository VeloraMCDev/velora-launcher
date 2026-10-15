export function actionContrast(value: unknown) {
  const valid = typeof value === 'string' && /^#[a-f\d]{3}(?:[a-f\d]{3})?$/i.test(value);
  const source = valid ? value : '#6d6af5';
  const background = source.length === 4 ? '#' + [...source.slice(1)].map(channel => channel + channel).join('') : source;
  const channels = [1, 3, 5].map(offset => parseInt(background.slice(offset, offset + 2), 16) / 255)
    .map(channel => channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4);
  const luminance = channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
  const darkRatio = (luminance + 0.05) / 0.05;
  const lightRatio = 1.05 / (luminance + 0.05);
  return { background, foreground: darkRatio >= lightRatio ? '#000000' : '#ffffff', ratio: Math.max(darkRatio, lightRatio) };
}
