/*********** /auth/register ***********/

export enum AuthRegisterErrorCode {
  Other = "Other",
  MalformedEmail = "MalformedEmail",
  MalformedDisplayName = "MalformedDisplayName",
  MalformedPassword = "MalformedPassword",
  UserExists = "UserExists",
  PasswordPwned = "PasswordPwned",
}

export type AuthRegisterError = {
  code: AuthRegisterErrorCode,
  message: string,
}

export type AuthRegisterResponse = {
  status: string,
  error: AuthRegisterError | null,
}

export type AuthRegisterRequest = {
  email: string,
  display_name: string,
  password: string,
}

/*********** /auth/verify ***********/

export enum AuthVerifyErrorCode {
  Other = "Other",
  MalformedCode = "MalformedCode",
  IncorrectCode = "IncorrectCode",
  VerificationAborted = "VerificationAborted",
  Unauthenticated = "Unauthenticated",
  RateLimit = "RateLimit",
}

export type AuthVerifyError = {
  code: AuthVerifyErrorCode,
  message: string,
}

export type AuthVerifyResponse = {
  status: string,
  error: AuthVerifyError | null,
}

export type AuthVerifyRequest = {
  code: string,
}

/*********** /auth/login ***********/

export enum AuthLoginErrorCode {
  Other = "Other",
  MalformedEmail = "MalformedEmail",

  // No such user exists with the credentials the client gave
  // This could be either that no user with the email exists or that user's password is wrong
  NoSuchUser = "NoSuchUser",
}

export type AuthLoginError = {
  code: AuthLoginErrorCode,
  message: string,
}

export type AuthLoginResponse = {
  status: string,
  error: AuthLoginError | null,
}

export type AuthLoginRequest = {
  email: string,
  password: string,
}
