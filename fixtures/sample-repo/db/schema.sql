CREATE TABLE users (
  id INT PRIMARY KEY
);
CREATE TABLE sessions (
  id INT PRIMARY KEY,
  user_id INT REFERENCES users(id)
);
