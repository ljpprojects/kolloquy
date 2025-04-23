// @ts-ignore
const submit = document.getElementById("submit")!!

// @ts-ignore
const email = document.getElementById("email") as HTMLInputElement

// @ts-ignore
const password = document.getElementById("password") as HTMLInputElement

const login = async () => {
    const data = {
        email: email.value,
        password: CryptoJS.SHA256(password.value).toString(CryptoJS.enc.Base64),
        redirect: "https://google.com"
    };


    let result = await fetch("./auth", {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify(data)
    })

    window.location.href = "./account"
}

submit.addEventListener("click", login)