export function validateToken(token: string): string {
  return token.replace('Bearer ', '');
}