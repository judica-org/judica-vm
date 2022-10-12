// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Send } from "@mui/icons-material";
import { FormControl, FormGroup, IconButton, TextField } from "@mui/material";
import React from "react";
import { tauri_host } from "../tauri_host";

export function NewChat() {
    const [text, set_text] = React.useState<string>("");
    const press_send = (ev: React.MouseEvent<HTMLButtonElement, MouseEvent>): void => {
        console.log("Sending ", text);
        tauri_host.send_chat(text);
        set_text("");

    };
    const press_enter = (ev: React.KeyboardEvent<HTMLDivElement>): void => {
        if (ev.key !== "Enter") return;
        console.log("Sending ", text);
        tauri_host.send_chat(text);
        set_text("");
    };
    return <div className="NewMessage">
        <FormControl>
            <FormGroup row>
                <TextField
                    size="medium"
                    multiline={text.length > 80}
                    onKeyPress={press_enter}
                    label="New Message" type="textarea" onChange={(ev) => set_text(ev.target.value)} value={text}></TextField>
                <IconButton type="submit" onClick={press_send}><Send></Send></IconButton>
            </FormGroup>
        </FormControl>
    </div>
}