import { validateToken } from './token.js';
import { loadUser } from '../users/store.js';

export function authMiddleware(request: Request): boolean {
  const token = request.headers.get('authorization');
  if (!token) return false;
  const userId = validateToken(token);
  return Boolean(loadUser(userId));
}