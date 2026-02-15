import type { AuthVerifyRequest, AuthVerifyResponse } from "./api";

(async () => {
  const { AuthVerifyErrorCode } = await import("./api.js");

  const validColour = "#6acc4f";
  const invalidColour = "#ee4352";

  const form: HTMLFormElement = document.querySelector("#register-form")!;
  const submitButton = document.getElementById("submit")!;
  const resendButton = document.getElementById("resend")!;

  const errorMessage = document.getElementById("error-msg")!;

  const digits: HTMLInputElement[] = [
    document.querySelector("#dig1")!,
    document.querySelector("#dig2")!,
    document.querySelector("#dig3")!,
    document.querySelector("#dig4")!,
    document.querySelector("#dig5")!,
    document.querySelector("#dig6")!,
  ];

  for (let i = 0; i < 6; i++) {
    if (i < 5) {
      digits[i].addEventListener("input", () => {
        digits[i + 1].focus();
      });
    }
  }

  resendButton.addEventListener("click", async () => {
    const headers = await fetch("/auth/resend", {
      credentials: "same-origin",
      method: "POST",
    });

    const body = await headers.json();

    alert(`api call result: ${JSON.stringify(body, null, 4)}`);
  });

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

    const headers = await fetch("/auth/verify", {
      credentials: "same-origin",
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        code: digits.map((d) => d.value).reduce((e, acc) => `${e}${acc}`, ""),
      } as AuthVerifyRequest),
    });

    const body = (await headers.json()) as AuthVerifyResponse;

    // If the API call succeeded, redirect to index page
    if (body.status === "success") {
      window.location.href = "/";
    }

    // Check for error
    if (body.status === "error" && body.error != null) {
      switch (body.error.code) {
        case AuthVerifyErrorCode.Unauthenticated:
          // Invalid credentials
          // TODO: handle error
          console.error("Unauthenticated");
          console.error(body.error.message);
          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          setTimeout(() => (window.location.href = "/register"), 5000);

          break;
        case AuthVerifyErrorCode.IncorrectCode:
          // Invalid credentials
          // TODO: handle error
          console.error("Incorrect code");
          console.error(body.error.message);
          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          break;
        case AuthVerifyErrorCode.VerificationAborted:
          // Invalid credentials
          // TODO: handle error
          console.error("Verification aborted");
          console.error(body.error.message);
          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          setTimeout(() => (window.location.href = "/register"), 5000);

          break;
        case AuthVerifyErrorCode.RateLimit:
          // Server error
          // TODO: handle error
          console.error("Rate limit");
          console.error(body.error.message);

          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          break;
        case AuthVerifyErrorCode.Other:
          // Server error
          // TODO: handle error
          console.error("Other");
          console.error(body.error.message);

          errorMessage.textContent = body.error.message;
          errorMessage.style.display = "block";

          break;
        // Malformed errors is not possible (it is never sent by the server)
      }
    }
  });

  digits[0].addEventListener("input", (e) => {
    console.log(e);

    // @ts-ignore
    if (e.inputType === "insertFromPaste") {
      const paste = digits[0].value;

      for (let i = 0; i < 6; i++) {
        digits[i].valueAsNumber = parseInt(paste[i]);
      }
    }
  });

  document.addEventListener("DOMContentLoaded", () => {
    digits[0].focus();
  });
})();
