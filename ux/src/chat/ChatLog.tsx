import { appWindow } from "@tauri-apps/api/window";
import React from "react";

export function ChatLog() {

    const [chat_log, set_chat_log] = React.useState<[number, number, string][]>([]);
    React.useEffect(() => {
        const unlisten = appWindow.listen("chat-log", (ev) => {
            console.log("Chat:", ev.payload);
            const new_msgs = ev.payload as typeof chat_log;
            set_chat_log(new_msgs)
        })
        return () => {
            (async () => {
                (await unlisten)()
            })();
        }
    });

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