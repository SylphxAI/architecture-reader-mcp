<?php
namespace App\Auth;

use App\Shared\Clock;

class TokenService
{
    public function issueToken(string $user): string
    {
        return $this->helperSalt() . ':' . $user;
    }

    private function helperSalt(): string
    {
        return 's3cret';
    }
}
