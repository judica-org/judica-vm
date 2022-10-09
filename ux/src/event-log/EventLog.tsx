import { Divider, Typography } from "@mui/material";
import { useState, useEffect, useRef } from "react";
import { LogEvent } from "../Types/Gameboard";
import './Events.css';

export function EventLog({ game_event_log }: { game_event_log: [number, LogEvent][] }) {
    const bottomEl = useRef<HTMLDivElement>(null);
    const [event_messages, set_event_messages] = useState<[number, LogEvent][]>([]);

    useEffect(() => {
        set_event_messages(game_event_log as [number, LogEvent][]);
        bottomEl.current?.scrollIntoView(false)
    }, [event_messages])

    return <div>
        <Typography className="EventsTitle"variant="h4" sx={{ textDecoration: 'underline' }}>Game Events</Typography>
        <Divider />
        <div className="EventLogScrollBox">
            {event_messages && event_messages.map(([a, b]) => {
                return <div key={a} className="EventMessage">
                    <Typography variant="body1" sx={{ textDecoration: 'underline' }} display="inline">{a}:</Typography>
                    <Typography variant="body2" sx={{ wordBreak: "break-word" }}>{JSON.stringify(b)}</Typography>
                </div>
            })}
            <div ref={bottomEl}></div>
        </div>
    </div>;
}