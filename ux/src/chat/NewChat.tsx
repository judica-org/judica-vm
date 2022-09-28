import { Button, FormControl, TextField } from "@mui/material";
import React from "react";
import { tauri_host } from "../tauri_host";

export function NewChat() {
    const [text, set_text] = React.useState<string>("");
    const newLocal = (ev: React.MouseEvent<HTMLButtonElement, MouseEvent>): void => {
        console.log("Sending ", text);
        tauri_host.send_chat(text);
        set_text("");

    };
    return <div className="NewMessage">
        <FormControl>
            <TextField label="New Message" type="textarea" onChange={(ev) => set_text(ev.target.value)} value={text}></TextField>
            <Button type="submit" onClick={
                newLocal
            }>Send</Button>
        </FormControl>
    </div>
}