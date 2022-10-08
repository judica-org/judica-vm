import { ChatLog } from "./ChatLog";
import { NewChat } from "./NewChat";
import "./Chat.css";
export function Chat({ chat_log }: { chat_log: [number, string, string][] }) {

    return <div className="Chat">
        <ChatLog chat_log={chat_log}></ChatLog>
        <NewChat></NewChat>
    </div>;
}