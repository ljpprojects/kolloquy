interface KolloquyAuthor {
    avatar: string,
    id: string,
    is_self: boolean,
    handle: string,
}

interface KolloquyMessageData {
    action: "PUT" | "EDIT" | "DELETE"
    content: string,
    author: KolloquyAuthor,
    chat: string,
}

const chatID = (document.getElementById("chatid")!! as HTMLDataElement).value;
const author: KolloquyAuthor = JSON.parse((document.getElementById("author")!! as HTMLDataElement).value);

const socket = new WebSocket("wss://kolloquy.com/chatws");

const messages = document.getElementById("info")!! as HTMLDivElement;
const sendButton = document.getElementById("send")!! as HTMLButtonElement;
const messageInput = document.getElementById("messageInput")!! as HTMLInputElement;

sendButton.addEventListener("click", () => {
    const data = {
        content: messageInput.value,
        action: "PUT",
        author,
        chat: chatID
    }

    socket.send(JSON.stringify(data))

    messageInput.value = ""
})

socket.addEventListener("message", (e) => {
    const data = JSON.parse(e.data) as KolloquyMessageData

    console.log(data)

    if (data.chat != chatID) {
        return false
    }

    switch (data.action) {
        case "PUT":
            const div = document.createElement("div")

            div.classList.add("chat")

            if (data.author.is_self || data.author.id == author.id) {
                div.style.marginLeft = "5vw"
            } else {
                div.style.marginRight = "5vw"
            }

            const div2 = document.createElement("div")

            div2.style.display = "grid"
            div2.style.justifyContent = "left"
            div2.style.alignItems = "center"
            div2.style.textAlign = "left"
            div2.style.verticalAlign = "central"
            div2.style.marginTop = "auto"
            div2.style.marginLeft = "1vmin"

            div2.innerHTML = `<b style="margin: 0">${data.author.handle}</b><p style="margin: 0">${data.content}</p>`

            const avatar = document.createElement("svg")

            avatar.innerHTML = data.author.avatar

            div.appendChild(div2)
            div.appendChild(avatar)

            messages.append(div)

            break;
    }

    return false
})