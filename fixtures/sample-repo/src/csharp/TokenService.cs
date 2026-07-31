using System.Collections.Generic;

namespace Sample.Auth
{
    public class TokenService
    {
        public string IssueToken(string user)
        {
            return HelperSalt() + ":" + user;
        }

        private string HelperSalt()
        {
            return "s3cret";
        }
    }
}
