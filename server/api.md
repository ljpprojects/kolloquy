# Backend API

## Registering
`POST` https://api.kolloquy.com/auth/register

### Request

```json5
{
  /* required */ "email": "xxx@foo.bar",
  /* required */ "password": "3875034e17855bac03a3cc9e107b1d28a9b44313d381c3335588525b4e70b55b",
  /* required */ "handle": "xxx",
  /* required */ "age": 19,
}
```

### Response

```json5
{
  "success": false,
  
  /* Only sent if success = false */
  "error": {
    "code": 100,
    "message": "Request timed out",
  }
}
```

**Headers**
* `Set-Cookie: __Secure-SSID=<SSID>; SameSite=Strict; Secure; HttpOnly; Max-Age=1200; Path=/auth; Domain=api.kolloquy.com`