// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Divider, Typography } from "@mui/material";
import { useState, useEffect, useRef } from "react";
import { EntityID } from "../Types/Gameboard";

export function ChatLog({ chat_log, nicks }: { chat_log: [number, string, string][], nicks: Record<EntityID, string> }) {
    const bottomEl = useRef<HTMLDivElement>(null);
    const [chat_messages, set_chat_messages] = useState<[number, string, string][]>([]);

    useEffect(() => {
        set_chat_messages(chat_log as [number, string, string][]);
        bottomEl.current?.scrollIntoView(false)
    }, [chat_log])

    return <div>
        <Typography className="ChatTitle" variant="h4" >Chat</Typography>
        <Divider />
        <div className="ChatLogScrollBox">
            {chat_messages && chat_messages.map(([a, b, c]) => {
                return <div key={a} className="ChatMessage">
                    <Typography variant="body2" sx={{ wordBreak: "break-word" }}><Typography variant="body1" sx={{ textDecoration: 'underline' }} display="inline">{nicks[b] ? `${nicks[b]}(${b})` : b}:</Typography> {c}</Typography>
                </div>
            })}
            <div ref={bottomEl}></div>
        </div>
    </div>;
}