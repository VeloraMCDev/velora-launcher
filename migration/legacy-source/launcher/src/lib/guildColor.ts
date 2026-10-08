// The same colour the live map gives a guild: hue from Java's String.hashCode of the guild id.
export function guildHue(id: string): number {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (Math.imul(31, h) + id.charCodeAt(i)) | 0;
  return ((h % 360) + 360) % 360;
}
