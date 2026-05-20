var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
(() => __awaiter(void 0, void 0, void 0, function* () {
    const { AuthLoginErrorCode } = yield import("./api.js");
    const form = document.querySelector("#login-form");
    const submitButton = document.getElementById("submit");
    const errorMessage = document.getElementById("error-msg");
    const email = document.querySelector("#email");
    const password = document.querySelector("#password");
    const validColour = "#6acc4f";
    const invalidColour = "#ee4352";
    form.addEventListener("input", () => {
        if (form.checkValidity()) {
            submitButton.style.backgroundColor = validColour;
        }
        else {
            submitButton.style.backgroundColor = invalidColour;
        }
    });
    form.addEventListener("submit", (e) => __awaiter(void 0, void 0, void 0, function* () {
        e.preventDefault();
        errorMessage.style.display = "none";
        const headers = yield fetch("/auth/login", {
            credentials: "same-origin",
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                email: email.value,
                password: password.value,
            }),
        });
        const body = (yield headers.json());
        if (body.status === "success") {
            window.location.href = "/";
        }
        if (body.status === "error" && body.error != null) {
            switch (body.error.code) {
                case AuthLoginErrorCode.NoSuchUser:
                    console.error("Invalid credentials");
                    console.error(body.error.message);
                    errorMessage.textContent = body.error.message;
                    errorMessage.style.display = "block";
                    break;
                case AuthLoginErrorCode.Other:
                    console.error("Server error");
                    console.error(body.error.message);
                    errorMessage.textContent = body.error.message;
                    errorMessage.style.display = "block";
                    break;
            }
        }
    }));
}))();
