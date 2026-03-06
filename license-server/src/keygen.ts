// Charset without ambiguous characters (no 0/O, 1/I/L)
const CHARSET = 'ABCDEFGHJKMNPQRSTUVWXYZ23456789';

export function generateConvxKey(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  const segments: string[] = [];
  for (let i = 0; i < 4; i++) {
    let seg = '';
    for (let j = 0; j < 4; j++) {
      seg += CHARSET[bytes[i * 4 + j]! % CHARSET.length];
    }
    segments.push(seg);
  }
  return `CONVX-${segments.join('-')}`;
}

export function generateDiscountCode(): string {
  const bytes = new Uint8Array(8);
  crypto.getRandomValues(bytes);
  const segments: string[] = [];
  for (let i = 0; i < 2; i++) {
    let seg = '';
    for (let j = 0; j < 4; j++) {
      seg += CHARSET[bytes[i * 4 + j]! % CHARSET.length];
    }
    segments.push(seg);
  }
  return `TOOLBOX-${segments.join('-')}`;
}
