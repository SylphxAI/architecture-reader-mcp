package com.example;

import java.util.HashMap;

public class TokenService {
    public String issueToken(String user) {
        return helperSalt() + ":" + user;
    }

    private String helperSalt() {
        return "s3cret";
    }
}
