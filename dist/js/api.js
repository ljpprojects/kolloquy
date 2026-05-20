export var AuthRegisterErrorCode;
(function (AuthRegisterErrorCode) {
    AuthRegisterErrorCode["Other"] = "Other";
    AuthRegisterErrorCode["MalformedEmail"] = "MalformedEmail";
    AuthRegisterErrorCode["MalformedDisplayName"] = "MalformedDisplayName";
    AuthRegisterErrorCode["MalformedPassword"] = "MalformedPassword";
    AuthRegisterErrorCode["UserExists"] = "UserExists";
    AuthRegisterErrorCode["PasswordPwned"] = "PasswordPwned";
})(AuthRegisterErrorCode || (AuthRegisterErrorCode = {}));
export var AuthVerifyErrorCode;
(function (AuthVerifyErrorCode) {
    AuthVerifyErrorCode["Other"] = "Other";
    AuthVerifyErrorCode["MalformedCode"] = "MalformedCode";
    AuthVerifyErrorCode["IncorrectCode"] = "IncorrectCode";
    AuthVerifyErrorCode["VerificationAborted"] = "VerificationAborted";
    AuthVerifyErrorCode["Unauthenticated"] = "Unauthenticated";
    AuthVerifyErrorCode["RateLimit"] = "RateLimit";
})(AuthVerifyErrorCode || (AuthVerifyErrorCode = {}));
export var AuthLoginErrorCode;
(function (AuthLoginErrorCode) {
    AuthLoginErrorCode["Other"] = "Other";
    AuthLoginErrorCode["MalformedEmail"] = "MalformedEmail";
    AuthLoginErrorCode["NoSuchUser"] = "NoSuchUser";
})(AuthLoginErrorCode || (AuthLoginErrorCode = {}));
