export function dimensions(width, height, fontSize) {
  return { columns: Math.min(400, Math.max(1, Math.floor((width - 12) / (fontSize * 0.6)))), rows: Math.min(240, Math.max(1, Math.floor((height - 12) / (fontSize * 1.2)))) };
}
export function drawFrame(canvas, frame, width, height, scale = 1) {
  const config = frame.config;
  const cellWidth = config.fontSize * 0.6;
  const cellHeight = config.fontSize * 1.2;
  canvas.width = Math.max(1, Math.round(width * scale));
  canvas.height = Math.max(1, Math.round(height * scale));
  const context = canvas.getContext('2d');
  context.scale(scale, scale);
  context.clearRect(0, 0, width, height);
  context.fillStyle = config.background;
  context.globalAlpha = Math.max(0.004, config.opacity);
  context.fillRect(0, 0, width, height);
  context.globalAlpha = 1;
  context.textBaseline = 'middle';
  for (const cell of frame.cells) {
    const x = 6 + cell.x * cellWidth, y = 6 + cell.y * cellHeight;
    context.save();
    context.beginPath(); context.rect(x, y, cellWidth * cell.width, cellHeight); context.clip();
    if (cell.bg) { context.fillStyle = cell.bg; context.fillRect(x, y, cellWidth * cell.width, cellHeight); }
    context.font = `${cell.italic ? 'italic ' : ''}${cell.bold ? '600' : '400'} ${config.fontSize}px Cascadia, 'Microsoft YaHei UI', 'PingFang SC', monospace`;
    context.fillStyle = cell.fg;
    context.fillText(cell.text, x, y + cellHeight / 2, cellWidth * cell.width);
    if (cell.underline) context.fillRect(x, y + cellHeight - 2, cellWidth * cell.width, 1);
    context.restore();
  }
}
