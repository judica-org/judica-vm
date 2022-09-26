import { appWindow } from "@tauri-apps/api/window";
import React from "react";

export function ChatLog({chat_log}:{chat_log:[number, number, string][]}) {


    const msgs = chat_log.map(([a, b, c]) => {
        return <div key={a}>
            <h6>{b}:</h6>
            <p>{c}</p>
        </div>
    });

    return <div>
        <div>
            <h3>Chat:</h3>
        </div>
        <div className="ChatLogScrollBox">
            {msgs}
        </div>

    </div>;
}