const users = new Map<string, { id: string }>();

export function loadUser(id: string) {
  return users.get(id);
}