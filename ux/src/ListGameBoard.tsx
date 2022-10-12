// Copyright Judica, Inc 2022
//
// This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at https://mozilla.org/MPL/2.0/.

import { Typography } from '@mui/material';
import React from 'react';
import { GameBoard } from './Types/Gameboard';

export function ListGameBoard(props: { g: GameBoard | null; }) {
  return props.g &&
    <div className='BoardJson'>
      <Typography component={"textarea"}

        style={{ width: "100%", minHeight: '75vh' }}
      >
        {JSON.stringify(props.g, null, 2)}
      </Typography>
    </div>
}
