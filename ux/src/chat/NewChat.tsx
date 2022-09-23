import { Button, FormControl, TextField } from "@mui/material";
import React from "react";
import { tauri_host } from "../tauri_host";

export function NewChat() {
    const [text, set_text] = React.useState<string>("");
    const newLocal = (ev: React.FormEvent<HTMLDivElement>): void => {

        ev.preventDefault();
        tauri_host.send_chat(text);
        set_text("");

    };
    return <div className="NewMessage">
        <FormControl onSubmit={newLocal}>
            <TextField label="New Message" type="textarea" onChange={(ev) => set_text(ev.target.value)} value={text}></TextField>
            <Button type="submit">Send</Button>
        </FormControl>
    </div>
}