import type { AuthRegisterRequest, AuthRegisterResponse } from "./api";

(async () => {
  const { AuthRegisterErrorCode } = await import("./api.js");

  const validColour = "#6acc4f";
  const invalidColour = "#ee4352";

  const form: HTMLFormElement = document.querySelector("#register-form")!;
  const submitButton = document.getElementById("submit")!;

  const email: HTMLInputElement = document.querySelector("#email")!;
  const displayName: HTMLInputElement =
    document.querySelector("#display-name")!;
  const password: HTMLInputElement = document.querySelector("#password")!;

  const errorMessage = document.getElementById("error-msg")!;

  form.addEventListener("input", () => {
    if (form.checkValidity()) {
      submitButton.style.backgroundColor = validColour;
    } else {
      submitButton.style.backgroundColor = invalidColour;
    }
  });

  form.addEventListener("submit", async (e) => {
    e.preventDefault();

    errorMessage.style.display = "none";

    const headers = await fetch("/auth/register", {
      credentials: "same-origin",
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        email: email.value,
        display_name: displayName.value,
        password: password.value,
      } as AuthRegisterRequest),
    });

    const body = (await headers.json()) as AuthRegisterResponse;

    // If the API call succeeded, redirect to verify page
    if (body.status === "success") {
      window.location.href = "/verify";
    }

    // Check for error
    if (body.status === "error" && body.error != null) {
      switch (body.error.code) {
        case AuthRegisterErrorCode.PasswordPwned:
          // Invalid credentials
          // TODO: handle error
          console.error("Password pwned");
          console.error(body.error.message);
          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          break;
        case AuthRegisterErrorCode.UserExists:
          // Invalid credentials
          // TODO: handle error
          console.error("User exists");
          console.error(body.error.message);
          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          break;
        case AuthRegisterErrorCode.Other:
          // Server error
          // TODO: handle error
          console.error("Server error");
          console.error(body.error.message);

          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          break;
        // Malformed errors is not possible (it is never sent by the server)
      }
    }
  });
})();
