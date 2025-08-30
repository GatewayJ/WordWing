import { validateDatePart, validateTimePart } from "../lib/datetimeInput";

type Props = {
  idPrefix: string;
  dateStr: string;
  timeStr: string;
  onDateStrChange: (v: string) => void;
  onTimeStrChange: (v: string) => void;
  dateLabel?: string;
  timeLabel?: string;
  optionalHint?: string;
};

/**
 * 纯文本日期 + 时间，避免 WebKitGTK 上原生 datetime-local 日历弹层阻塞整页交互。
 */
export function PlainDateTimeFields({
  idPrefix,
  dateStr,
  timeStr,
  onDateStrChange,
  onTimeStrChange,
  dateLabel = "日期",
  timeLabel = "时间",
  optionalHint = "可选；时间留空为当天 0:00",
}: Props) {
  const dateInvalid = dateStr.length > 0 && !validateDatePart(dateStr);
  const timeInvalid = timeStr.length > 0 && !validateTimePart(timeStr);

  return (
    <div className="plain-datetime">
      <div className="plain-datetime__row">
        <div className="plain-datetime__field">
          <label className="settings-label" htmlFor={`${idPrefix}-date`}>
            {dateLabel}
          </label>
          <input
            id={`${idPrefix}-date`}
            className="todo-input"
            type="text"
            inputMode="numeric"
            autoComplete="off"
            placeholder="YYYY-MM-DD"
            value={dateStr}
            onChange={(e) => onDateStrChange(e.target.value)}
            aria-invalid={dateInvalid}
          />
        </div>
        <div className="plain-datetime__field">
          <label className="settings-label" htmlFor={`${idPrefix}-time`}>
            {timeLabel}
          </label>
          <input
            id={`${idPrefix}-time`}
            className="todo-input"
            type="text"
            inputMode="numeric"
            autoComplete="off"
            placeholder="HH:MM"
            value={timeStr}
            onChange={(e) => onTimeStrChange(e.target.value)}
            aria-invalid={timeInvalid}
          />
        </div>
      </div>
      <p className="plain-datetime__hint cell-muted">{optionalHint}</p>
    </div>
  );
}
