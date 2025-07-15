import { AlertCircle, Send } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { FileSearchTextarea } from '@/components/ui/file-search-textarea';
import { useContext, useMemo, useState } from 'react';
import { makeRequest } from '@/lib/api.ts';
import {
  TaskAttemptDataContext,
  TaskDetailsContext,
  TaskSelectedAttemptContext,
} from '@/components/context/taskDetailsContext.ts';

export function TaskFollowUpSection() {
  const { task, projectId } = useContext(TaskDetailsContext);
  const { selectedAttempt } = useContext(TaskSelectedAttemptContext);
  const { attemptData, fetchAttemptData, isAttemptRunning } = useContext(
    TaskAttemptDataContext
  );

  const [followUpMessage, setFollowUpMessage] = useState('');
  const [isSendingFollowUp, setIsSendingFollowUp] = useState(false);
  const [followUpError, setFollowUpError] = useState<string | null>(null);

  const canSendFollowUp = useMemo(() => {
    if (
      !selectedAttempt ||
      attemptData.activities.length === 0 ||
      isAttemptRunning ||
      isSendingFollowUp
    ) {
      return false;
    }

    const codingAgentActivities = attemptData.activities.filter(
      (activity) => activity.status === 'executorcomplete'
    );

    return codingAgentActivities.length > 0;
  }, [
    selectedAttempt,
    attemptData.activities,
    isAttemptRunning,
    isSendingFollowUp,
  ]);

  const onSendFollowUp = async () => {
    if (!task || !selectedAttempt || !followUpMessage.trim()) return;

    try {
      setIsSendingFollowUp(true);
      setFollowUpError(null);
      const response = await makeRequest(
        `/api/projects/${projectId}/tasks/${selectedAttempt.task_id}/attempts/${selectedAttempt.id}/follow-up`,
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            prompt: followUpMessage.trim(),
          }),
        }
      );

      if (response.ok) {
        setFollowUpMessage('');
        fetchAttemptData(selectedAttempt.id, selectedAttempt.task_id);
      } else {
        const errorText = await response.text();
        setFollowUpError(
          `Failed to start follow-up execution: ${
            errorText || response.statusText
          }`
        );
      }
    } catch (err) {
      setFollowUpError(
        `Failed to send follow-up: ${
          err instanceof Error ? err.message : 'Unknown error'
        }`
      );
    } finally {
      setIsSendingFollowUp(false);
    }
  };

  return (
    selectedAttempt && (
      <div className="border-t p-4">
        <div className="space-y-2">
          {followUpError && (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{followUpError}</AlertDescription>
            </Alert>
          )}
          <div className="flex gap-2 items-start">
            <FileSearchTextarea
              placeholder="Ask a follow-up question... Type @ to search files."
              value={followUpMessage}
              onChange={(value) => {
                setFollowUpMessage(value);
                if (followUpError) setFollowUpError(null);
              }}
              onKeyDown={(e) => {
                if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
                  e.preventDefault();
                  if (
                    canSendFollowUp &&
                    followUpMessage.trim() &&
                    !isSendingFollowUp
                  ) {
                    onSendFollowUp();
                  }
                }
              }}
              className="flex-1 min-h-[40px] resize-none"
              disabled={!canSendFollowUp}
              projectId={projectId}
              rows={1}
            />
            <Button
              onClick={onSendFollowUp}
              disabled={
                !canSendFollowUp || !followUpMessage.trim() || isSendingFollowUp
              }
              size="sm"
            >
              {isSendingFollowUp ? (
                <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-current" />
              ) : (
                <>
                  <Send className="h-4 w-4 mr-2" />
                  Send
                </>
              )}
            </Button>
          </div>
        </div>
      </div>
    )
  );
}
