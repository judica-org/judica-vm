import { appWindow } from "@tauri-apps/api/window";
import React from "react";

export function ChatLog() {

    const [chat_log, set_chat_log] = React.useState<[number, number, string][]>([]);
    React.useEffect(() => {
        const unlisten = appWindow.listen("chat-log", (ev) => {
            console.log(ev.payload);
            const new_keys = ev.payload as typeof chat_log;
            set_chat_log(chat_log)
        })
        return () => {
            (async () => {
                (await unlisten)()
            })();
        }
    });


    return <div>
        <div>
            <h3>Chat:</h3>
        </div>
        <div className="ChatLogScrollBox">
            {chat_log.map(([a, b, c]) =>
                <div key={a}>
                    <h6>{b}:</h6>
                    <p>{c}</p>
                </div>)}
        </div>

    </div>;
}