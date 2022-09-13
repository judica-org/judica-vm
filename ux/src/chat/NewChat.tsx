import React from "react";
import { tauri_host } from "../tauri_host";

export function NewChat() {
    const [text, set_text] = React.useState<string>("");
    const newLocal = (ev: React.FormEvent<HTMLFormElement>): void => {
        ev.preventDefault();
        tauri_host.send_chat(text);
        set_text("");
    };
    return <div className="NewMessage">
        <form onSubmit={newLocal}>
            <label>New Message</label>
            <input type="textarea" onChange={(ev) => set_text(ev.target.value)}></input>
            <button type="submit">Send</button>
        </form>
    </div>
}