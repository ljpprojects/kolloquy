import type { AuthLoginRequest, AuthLoginResponse } from "./api";

(async () => {
  const { AuthLoginErrorCode } = await import("./api.js");

  const form: HTMLFormElement = document.querySelector("#login-form")!;
  const submitButton = document.getElementById("submit")!;

  const errorMessage = document.getElementById("error-msg")!;

  const email: HTMLInputElement = document.querySelector("#email")!;
  const password: HTMLInputElement = document.querySelector("#password")!;

  const validColour = "#6acc4f";
  const invalidColour = "#ee4352";

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

    const headers = await fetch("/auth/login", {
      credentials: "same-origin",
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        email: email.value,
        password: password.value,
      } as AuthLoginRequest),
    });

    const body = (await headers.json()) as AuthLoginResponse;

    // If the API call succeeded, redirect to index page
    if (body.status === "success") {
      window.location.href = "/";
    }

    // Check for error
    if (body.status === "error" && body.error != null) {
      switch (body.error.code) {
        case AuthLoginErrorCode.NoSuchUser:
          // Invalid credentials
          // TODO: handle error
          console.error("Invalid credentials");
          console.error(body.error.message);
          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          break;
        case AuthLoginErrorCode.Other:
          // Server error
          // TODO: handle error
          console.error("Server error");
          console.error(body.error.message);

          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          break;
        // Malformed email is not possible (it is never sent by the server)
      }
    }
  });
})();
