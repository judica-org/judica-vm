import { SwitchToDB } from './SwitchToDB';
import { SwitchToGame } from './SwitchToGame';
import { KeySelector } from './KeySelector';
import { MakeNewChain } from './MakeNewChain';
import "./AppHeader.css";

export function AppHeader() {
  return <div className="App-header">
    <SwitchToGame></SwitchToGame>
    <MakeNewChain></MakeNewChain>
    <SwitchToDB></SwitchToDB>
    <KeySelector></KeySelector>
  </div>;
}
