import { Typography } from "@mui/material";
import { appWindow } from "@tauri-apps/api/window";
import React from "react";

export function ChatLog({ chat_log }: { chat_log: [number, number, string][] }) {


    const msgs = chat_log.map(([a, b, c]) => {
        return <div key={a} className="ChatMessage">

            <Typography variant="body2"><Typography variant="h6" sx={{ textDecoration: 'underline' }} display="inline">{b}:</Typography> {c}</Typography>
        </div>
    });

    return <div>
        <div>
            <Typography variant="h4">Chat:</Typography>
        </div>
        <div className="ChatLogScrollBox">
            {msgs}
        </div>
    </div>;
}