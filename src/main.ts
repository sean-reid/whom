import { localIsoDate, puzzleNumber } from "../shared/day.ts";

const issue = document.getElementById("issue");
const today = localIsoDate();
const n = puzzleNumber(today);
if (issue && n !== null) issue.textContent = `No. ${n}`;
