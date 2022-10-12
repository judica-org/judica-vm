// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { ChatLog } from "./ChatLog";
import { NewChat } from "./NewChat";
import "./Chat.css";
export function Chat({ chat_log }: { chat_log: [number, string, string][] }) {

    return <div className="Chat">
        <ChatLog chat_log={chat_log}></ChatLog>
        <NewChat></NewChat>
    </div>;
}