const chatCreate = document.getElementById('chatcreate');
const participantsList = document.getElementById('inlineParticipants');
const chatName = document.getElementById('chatname') as HTMLInputElement;
const participantInput = document.getElementById('participantInput') as HTMLInputElement;

// @ts-ignore
const handleRegex = /^@?\w{3,15}$/;
const chatNameRegex = /^.{1,20}$/

participantInput.addEventListener("change", () => {
    if (!handleRegex.test(participantInput.value)) {
        return false;
    }

    const sanitised = DOMPurify.sanitize(participantInput.value);

    participantsList!!.innerHTML += `<p class="inlineParticipant">${sanitised[0] == "@" ? sanitised : "@" + sanitised}</p>`
})

chatCreate?.addEventListener("click", async () => {
    if (!chatNameRegex.test(chatName.value)) {
        return false;
    }

    const data = {
        participants: [...participantsList!!.children].filter(e => e.tagName == "p").map(e => e.textContent),
        name: chatName.value,
    }

    const result = await fetch("/create", {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify(data)
    })

    let json = await result.json();

    if (!json["success"]) {
        alert(JSON.stringify(json["error"], null, 4))
    }

    window.location.href = "./chat/" + json["id"];
})