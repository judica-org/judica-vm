import { Divider, Typography } from "@mui/material";

export function ChatLog({ chat_log }: { chat_log: [number, number, string][] }) {

    const msgs = chat_log.map(([a, b, c]) => {
        return <div key={a} className="ChatMessage">
            <Typography variant="body2"><Typography variant="h6" sx={{ textDecoration: 'underline' }} display="inline">{b}:</Typography> {c}</Typography>
        </div>
    });

    return <div>
        <Typography variant="h4" sx={{ textDecoration: 'underline' }}>Chat</Typography>
        <Divider />
        <div className="ChatLogScrollBox">
            {msgs}
        </div>
    </div>;
}