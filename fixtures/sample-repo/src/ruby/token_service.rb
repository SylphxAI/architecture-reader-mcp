require "json"

module Auth
  class TokenService
    def issue_token(user)
      helper_salt + ":" + user
    end

    def helper_salt
      "s3cret"
    end
  end
end
