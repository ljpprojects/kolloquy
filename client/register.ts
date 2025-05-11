const ageInput = document.getElementById("age") as HTMLInputElement
const ageDisplay = document.getElementById("age-display")!!

const emailRegex = /^(\(\w+\))?(([A-Za-z\d]+!)?\w([A-Za-z\d][-.\w]?)+[-A-Za-z\d]|"([-\] (),.:;<>@\[\w]|\\\\"?)+")(\/[A-Za-z\d]+)?(\+[A-Za-z\d]+)?(%(([A-Za-z\d]+)(\.[A-Za-z\d]+)*|\[((((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))|([A-Fa-f\d]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1?\d)?\d)\.){3}(25[0-5]|(2[0-4]|1?\d)?\d)|([A-Fa-f\d]{1,4}:){7}[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,7}:|([A-Fa-f\d]{1,4}:){1,6}:[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,5}(:[A-Fa-f\d]{1,4}){1,2}|([A-Fa-f\d]{1,4}:){1,4}(:[A-Fa-f\d]{1,4}){1,3}|([A-Fa-f\d]{1,4}:){1,3}(:[A-Fa-f\d]{1,4}){1,4}|([A-Fa-f\d]{1,4}:){1,2}(:[A-Fa-f\d]{1,4}){1,5}|[A-Fa-f\d]{1,4}:((:[A-Fa-f\d]{1,4}){1,6})|:((:[A-Fa-f\d]{1,4}){1,7}|:)|fe80:(:[A-Fa-f\d]{0,4}){0,4}%[A-Za-z\d]+|::(ffff(:0{1,4})?:)?((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2})|([A-Fa-f\d]{1,4}:){1,4}:((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))]))?(\(\w+\))?@(([A-Za-z\d]+)(\.[A-Za-z\d]+)*$|\[((((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))|([A-Fa-f\d]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1?\d)?\d)\.){3}(25[0-5]|(2[0-4]|1?\d)?\d)|([A-Fa-f\d]{1,4}:){7}[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,7}:|([A-Fa-f\d]{1,4}:){1,6}:[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,5}(:[A-Fa-f\d]{1,4}){1,2}|([A-Fa-f\d]{1,4}:){1,4}(:[A-Fa-f\d]{1,4}){1,3}|([A-Fa-f\d]{1,4}:){1,3}(:[A-Fa-f\d]{1,4}){1,4}|([A-Fa-f\d]{1,4}:){1,2}(:[A-Fa-f\d]{1,4}){1,5}|[A-Fa-f\d]{1,4}:((:[A-Fa-f\d]{1,4}){1,6})|:((:[A-Fa-f\d]{1,4}){1,7}|:)|fe80:(:[A-Fa-f\d]{0,4}){0,4}%[A-Za-z\d]+|::(ffff(:0{1,4})?:)?((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2})|([A-Fa-f\d]{1,4}:){1,4}:((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))])$/;
// @ts-ignore
const handleRegex = /^@?\w{3,15}$/;
const passwordRegex = /^(?=.*[^\W_\d])(?=.*[\d\p{N}])(?=.*[\p{P}\p{S}]).{8,}$/u;

ageInput?.addEventListener("input", () => {
    ageDisplay.textContent = ageInput.value
})

// @ts-ignore
const submit = document.getElementById("submit")!!

// @ts-ignore
const email = document.getElementById("email") as HTMLInputElement
const emailErrorMessage = document.getElementById("emailerr") as HTMLInputElement

// @ts-ignore
const password = document.getElementById("password") as HTMLInputElement
const passwordErrorMessage = document.getElementById("passerr") as HTMLInputElement

const handle = document.getElementById("handle") as HTMLInputElement
const handleErrorMessage = document.getElementById("handleerr") as HTMLInputElement

const invalidEmailError = "Invalid email address.";
const invalidHandleError = "Handles must be 3-15 characters in length and contain only alphanumeric characters, _, !, $, -, ., \\ and /.";
const invalidPasswordError = "Passwords must be 8 characters or longer and contain at least one letter, number, and special character or punctuation."

const emailError = (error: string) => {
    emailErrorMessage.textContent = error
    emailErrorMessage.classList.remove("hidden")
    email.classList.add("invalid")
    submit.setAttribute("disabled", "true")
}

const handleError = (error: string) => {
    handleErrorMessage.textContent = error
    handleErrorMessage.classList.remove("hidden")
    handle.classList.add("invalid")
    submit.setAttribute("disabled", "true")
}

const passwordError = (error: string) => {
    passwordErrorMessage.textContent = error
    passwordErrorMessage.classList.remove("hidden")
    password.classList.add("invalid")
    submit.setAttribute("disabled", "true")
}

if (!document.body.dataset.noCheck) {
    email.oninput = () => {
        if (!emailRegex.test(email.value)) {
            emailError(invalidEmailError)
        } else {
            email.classList.remove("invalid")
            emailErrorMessage.classList.add("hidden")
            submit.removeAttribute("disabled")
        }
    }
}

if (handle && !handle.dataset.noCheck) {
    handle.oninput = () => {
        if (!handleRegex.test(handle.value)) {
            handleError(invalidHandleError)
        } else {
            handle.classList.remove("invalid")
            handleErrorMessage.classList.add("hidden")
            submit.removeAttribute("disabled")
        }
    }
}

if (!password.dataset.noCheck) {
    password.oninput = () => {
        if (!passwordRegex.test(password.value)) {
            passwordError(invalidPasswordError)
        } else {
            password.classList.remove("invalid")
            passwordErrorMessage.classList.add("hidden")
            submit.removeAttribute("disabled")
        }
    }
}

const register = async () => {
    const data = {
        email: email.value,
        password: CryptoJS.SHA256(password.value).toString(CryptoJS.enc.Base64),
        handle: handle.value,
        age: Number(ageInput.value),
    };

    try {
        let result = await fetch("./register", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify(data)
        })

        const loginSection = document.getElementById("login")!!

        let json = await result.json();

        if (!json["success"]) {
            switch (json["error"]["code"]) {
                case 100:
                case 201:
                    emailError(json["error"]["message"])
                    break;

                case 101:
                case 202:
                    handleError(json["error"]["message"])
                    break;
            }
        }

        window.location.href = "./account"
    } catch (e) {
        alert(e)
    }
}

submit.addEventListener("click", register)