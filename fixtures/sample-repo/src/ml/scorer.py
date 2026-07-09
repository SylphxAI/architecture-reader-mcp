from auth.token import validate_token

def score_user(token: str) -> str:
    user_id = validate_token(token)
    return user_id