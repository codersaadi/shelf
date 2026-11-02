export function imageHasTransparency(image: HTMLImageElement): boolean {
  if (!image.naturalWidth || !image.naturalHeight) return false;

  const maxDimension = 64;
  const scale = Math.min(1, maxDimension / Math.max(image.naturalWidth, image.naturalHeight));
  const width = Math.max(1, Math.round(image.naturalWidth * scale));
  const height = Math.max(1, Math.round(image.naturalHeight * scale));
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;

  try {
    const context = canvas.getContext('2d', { willReadFrequently: true });
    if (!context) return false;
    context.drawImage(image, 0, 0, width, height);
    const pixels = context.getImageData(0, 0, width, height).data;
    for (let alpha = 3; alpha < pixels.length; alpha += 4) {
      if (pixels[alpha] < 255) return true;
    }
  } catch {
    return false;
  }

  return false;
}
