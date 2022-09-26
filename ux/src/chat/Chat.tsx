import { ChatLog } from "./ChatLog";
import { NewChat } from "./NewChat";
import "./Chat.css";
export function Chat({ chat_log }: { chat_log: [number, number, string][] }) {

    return <div className="Chat">
        <ChatLog chat_log={chat_log}></ChatLog>
        <NewChat></NewChat>
    </div>;
}