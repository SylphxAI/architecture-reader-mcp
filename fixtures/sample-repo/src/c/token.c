#include "token.h"
#include <stdio.h>

struct TokenBucket {
  int capacity;
};

static int helper_salt(int x) {
  return x + 1;
}

int issue_token(int seed) {
  return helper_salt(seed);
}
