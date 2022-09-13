import { ChatLog } from "./ChatLog";
import { NewChat } from "./NewChat";
import "./Chat.css";
export function Chat() {

    return <div className="Chat">
        <ChatLog></ChatLog>
        <NewChat></NewChat>
    </div>;
}