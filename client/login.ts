const authEndpoint = "https://auth.kolloquy.com/oauth/"

const ageInput = document.getElementById("age") as HTMLInputElement
const ageDisplay = document.getElementById("age-display")!!

const emailRegex = /^(\(\w+\))?(([A-Za-z\d]+!)?\w([A-Za-z\d][-.\w]?)+[-A-Za-z\d]|"([-\] (),.:;<>@\[\w]|\\\\"?)+")(\/[A-Za-z\d]+)?(\+[A-Za-z\d]+)?(%(([A-Za-z\d]+)(\.[A-Za-z\d]+)*|\[((((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))|([A-Fa-f\d]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1?\d)?\d)\.){3}(25[0-5]|(2[0-4]|1?\d)?\d)|([A-Fa-f\d]{1,4}:){7}[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,7}:|([A-Fa-f\d]{1,4}:){1,6}:[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,5}(:[A-Fa-f\d]{1,4}){1,2}|([A-Fa-f\d]{1,4}:){1,4}(:[A-Fa-f\d]{1,4}){1,3}|([A-Fa-f\d]{1,4}:){1,3}(:[A-Fa-f\d]{1,4}){1,4}|([A-Fa-f\d]{1,4}:){1,2}(:[A-Fa-f\d]{1,4}){1,5}|[A-Fa-f\d]{1,4}:((:[A-Fa-f\d]{1,4}){1,6})|:((:[A-Fa-f\d]{1,4}){1,7}|:)|fe80:(:[A-Fa-f\d]{0,4}){0,4}%[A-Za-z\d]+|::(ffff(:0{1,4})?:)?((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2})|([A-Fa-f\d]{1,4}:){1,4}:((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))]))?(\(\w+\))?@(([A-Za-z\d]+)(\.[A-Za-z\d]+)*$|\[((((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))|([A-Fa-f\d]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1?\d)?\d)\.){3}(25[0-5]|(2[0-4]|1?\d)?\d)|([A-Fa-f\d]{1,4}:){7}[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,7}:|([A-Fa-f\d]{1,4}:){1,6}:[A-Fa-f\d]{1,4}|([A-Fa-f\d]{1,4}:){1,5}(:[A-Fa-f\d]{1,4}){1,2}|([A-Fa-f\d]{1,4}:){1,4}(:[A-Fa-f\d]{1,4}){1,3}|([A-Fa-f\d]{1,4}:){1,3}(:[A-Fa-f\d]{1,4}){1,4}|([A-Fa-f\d]{1,4}:){1,2}(:[A-Fa-f\d]{1,4}){1,5}|[A-Fa-f\d]{1,4}:((:[A-Fa-f\d]{1,4}){1,6})|:((:[A-Fa-f\d]{1,4}){1,7}|:)|fe80:(:[A-Fa-f\d]{0,4}){0,4}%[A-Za-z\d]+|::(ffff(:0{1,4})?:)?((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2})|([A-Fa-f\d]{1,4}:){1,4}:((1?\d{1,2}|2[0-5]{1,2})\.){3}(1?\d{1,2}|2[0-5]{1,2}))])$/;
const handleRegex = /^@?[\w!$-.\\\/]{3,15}$/;
const passwordRegex = /^(?=.*[^\W_\d])(?=.*[\d\p{N}])(?=.*[\p{P}\p{S}]).{8,}$/u;

ageInput.addEventListener("input", () => {
    ageDisplay.textContent = ageInput.value
})

const email = document.getElementById("email") as HTMLInputElement

const password = document.getElementById("password") as HTMLInputElement
const passwordErrorMessage = document.getElementById("passerr") as HTMLInputElement

const handle = document.getElementById("handle") as HTMLInputElement
const handleErrorMessage = document.getElementById("handleerr") as HTMLInputElement

email.oninput = () => {
    if (!emailRegex.test(email.value)) {
        email.classList.add("invalid")
    } else {
        email.classList.remove("invalid")
    }
}

handle.oninput = () => {
    if (!handleRegex.test(handle.value)) {
        handle.classList.add("invalid")
        handleErrorMessage.classList.remove("hidden")
    } else {
        handle.classList.remove("invalid")
        handleErrorMessage.classList.add("hidden")
    }
}

password.oninput = () => {
    if (!passwordRegex.test(password.value)) {
        password.classList.add("invalid")
        passwordErrorMessage.classList.remove("hidden")
    } else {
        password.classList.remove("invalid")
        passwordErrorMessage.classList.add("hidden")
    }
}

const register = async () => {
    const data = {
        email: email.value,
        password: CryptoJS.SHA256(password.value).toString(CryptoJS.enc.Base64),
        handle: handle.value,
        age: ageDisplay.textContent,
    };

    console.log(data)
}