package com.example.auth

import kotlin.collections.List

class TokenService {
    fun issueToken(user: String): String {
        return helperSalt() + ":" + user
    }

    private fun helperSalt(): String {
        return "s3cret"
    }
}
