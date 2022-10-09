import { Divider, Typography } from "@mui/material";
import { useState, useEffect, useRef } from "react";
import { EntityID, LogEvent } from "../Types/Gameboard";
import './Events.css';

export function EventLog({ game_event_log }: { game_event_log: [number, EntityID, LogEvent][] }) {
    const bottomEl = useRef<HTMLDivElement>(null);
    const [event_messages, set_event_messages] = useState<[number, EntityID, LogEvent][]>([]);

    useEffect(() => {
        set_event_messages(game_event_log as [number, EntityID, LogEvent][]);
        bottomEl.current?.scrollIntoView(false)
    }, [event_messages])

    return <div>
        <Typography className="EventsTitle" variant="h4" >Game Events</Typography>
        <Divider />
        <div className="EventLogScrollBox">
            {event_messages && event_messages.map(([_, a, b]) => {
                return <div key={a} className="EventMessage">
                    <Typography className="LogUserName" variant="body1" sx={{ textDecoration: 'underline' }} display="inline">{a}:</Typography>
                    <Typography className="LogEvent" variant="body2" sx={{ wordBreak: "break-word" }}>  {JSON.stringify(b)}</Typography>
                </div>
            })}
            <div ref={bottomEl}></div>
        </div>
    </div>;
}