import { Divider, Typography } from "@mui/material";
import { useState, useEffect, useRef } from "react";
import { EntityID, LogEvent } from "../Types/Gameboard";
import './Events.css';

export function EventLog({ game_event_log }: { game_event_log: [number, EntityID, LogEvent][] }) {
    const bottomEl = useRef<HTMLDivElement>(null);
    const [event_messages, set_event_messages] = useState<[number, EntityID, LogEvent][]>([]);

    useEffect(() => {
        const filtered = game_event_log.filter(([_num, _player, logEvent]) => !JSON.stringify(logEvent).includes("heartbeat"))
        set_event_messages(filtered);
        bottomEl.current?.scrollIntoView(false)
    }, [game_event_log])

    return <div>
        <Typography className="EventsTitle" variant="h4" >Game Events</Typography>
        <Divider />
        <div className="EventLogScrollBox">
            {event_messages && event_messages.map(([a, b, c]) => {
                return <div key={a} className="EventMessage">
                    <Typography className="LogUserName" variant="body1" sx={{ textDecoration: 'underline' }} display="inline">{b}:</Typography>
                    <Typography className="LogEvent" variant="body2" sx={{ wordBreak: "break-word" }}>  {JSON.stringify(c)}</Typography>
                </div>
            })}
            <div ref={bottomEl}></div>
        </div>
    </div>;
}