def validate_token(token: str) -> str:
    return token.replace("Bearer ", "")