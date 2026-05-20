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
    const { AuthVerifyErrorCode } = yield import("./api.js");
    const validColour = "#6acc4f";
    const invalidColour = "#ee4352";
    const form = document.querySelector("#register-form");
    const submitButton = document.getElementById("submit");
    const resendButton = document.getElementById("resend");
    const errorMessage = document.getElementById("error-msg");
    const digits = [
        document.querySelector("#dig1"),
        document.querySelector("#dig2"),
        document.querySelector("#dig3"),
        document.querySelector("#dig4"),
        document.querySelector("#dig5"),
        document.querySelector("#dig6"),
    ];
    for (let i = 0; i < 6; i++) {
        if (i < 5) {
            digits[i].addEventListener("input", () => {
                digits[i + 1].focus();
            });
        }
    }
    resendButton.addEventListener("click", () => __awaiter(void 0, void 0, void 0, function* () {
        const headers = yield fetch("/auth/resend", {
            credentials: "same-origin",
            method: "POST",
        });
        const body = yield headers.json();
        alert(`api call result: ${JSON.stringify(body, null, 4)}`);
    }));
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
        const headers = yield fetch("/auth/verify", {
            credentials: "same-origin",
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                code: digits.map((d) => d.value).reduce((e, acc) => `${e}${acc}`, ""),
            }),
        });
        const body = (yield headers.json());
        if (body.status === "success") {
            window.location.href = "/";
        }
        if (body.status === "error" && body.error != null) {
            switch (body.error.code) {
                case AuthVerifyErrorCode.Unauthenticated:
                    console.error("Unauthenticated");
                    console.error(body.error.message);
                    errorMessage.textContent = body.error.message;
                    errorMessage.style.display = "block";
                    setTimeout(() => (window.location.href = "/register"), 5000);
                    break;
                case AuthVerifyErrorCode.IncorrectCode:
                    console.error("Incorrect code");
                    console.error(body.error.message);
                    errorMessage.textContent = body.error.message;
                    errorMessage.style.display = "block";
                    break;
                case AuthVerifyErrorCode.VerificationAborted:
                    console.error("Verification aborted");
                    console.error(body.error.message);
                    errorMessage.textContent = body.error.message;
                    errorMessage.style.display = "block";
                    setTimeout(() => (window.location.href = "/register"), 5000);
                    break;
                case AuthVerifyErrorCode.RateLimit:
                    console.error("Rate limit");
                    console.error(body.error.message);
                    errorMessage.textContent = body.error.message;
                    errorMessage.style.display = "block";
                    break;
                case AuthVerifyErrorCode.Other:
                    console.error("Other");
                    console.error(body.error.message);
                    errorMessage.textContent = body.error.message;
                    errorMessage.style.display = "block";
                    break;
            }
        }
    }));
    digits[0].addEventListener("input", (e) => {
        console.log(e);
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
}))();
