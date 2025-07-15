import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@radix-ui/react-label';
import { Textarea } from '@/components/ui/textarea.tsx';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@radix-ui/react-select';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useCallback, useContext, useEffect, useState } from 'react';
import {
  TaskDetailsContext,
  TaskSelectedAttemptContext,
} from '@/components/context/taskDetailsContext.ts';
import { makeRequest } from '@/lib/api.ts';
import { ProvidePatDialog } from '@/components/ProvidePatDialog';
import { ApiResponse, GitBranch } from 'shared/types.ts';

type Props = {
  showCreatePRDialog: boolean;
  setShowCreatePRDialog: (show: boolean) => void;
  creatingPR: boolean;
  setCreatingPR: (creating: boolean) => void;
  setError: (error: string | null) => void;
  branches: GitBranch[];
};

function CreatePrDialog({
  showCreatePRDialog,
  setCreatingPR,
  setShowCreatePRDialog,
  creatingPR,
  setError,
  branches,
}: Props) {
  const { projectId, task } = useContext(TaskDetailsContext);
  const { selectedAttempt } = useContext(TaskSelectedAttemptContext);
  const [prTitle, setPrTitle] = useState('');
  const [prBody, setPrBody] = useState('');
  const [prBaseBranch, setPrBaseBranch] = useState(
    selectedAttempt?.base_branch || 'main'
  );
  const [showPatDialog, setShowPatDialog] = useState(false);
  const [patDialogError, setPatDialogError] = useState<string | null>(null);

  useEffect(() => {
    if (showCreatePRDialog) {
      setPrTitle(`${task.title} (codecommand)`);
      setPrBody(task.description || '');
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [showCreatePRDialog]);

  // Update PR base branch when selected attempt changes
  useEffect(() => {
    if (selectedAttempt?.base_branch) {
      setPrBaseBranch(selectedAttempt.base_branch);
    }
  }, [selectedAttempt?.base_branch]);

  const handleConfirmCreatePR = useCallback(async () => {
    if (!projectId || !selectedAttempt?.id || !selectedAttempt?.task_id) return;

    try {
      setCreatingPR(true);
      const response = await makeRequest(
        `/api/projects/${projectId}/tasks/${selectedAttempt.task_id}/attempts/${selectedAttempt.id}/create-pr`,
        {
          method: 'POST',
          body: JSON.stringify({
            title: prTitle,
            body: prBody || null,
            base_branch: prBaseBranch || null,
          }),
        }
      );

      if (response.ok) {
        const result: ApiResponse<string> = await response.json();
        console.log(result);
        if (result.success && result.data) {
          // Open the PR URL in a new tab
          window.open(result.data, '_blank');
          setShowCreatePRDialog(false);
          // Reset form
          setPrTitle('');
          setPrBody('');
          setPrBaseBranch(selectedAttempt?.base_branch || 'main');
        } else if (result.message === 'insufficient_github_permissions') {
          setShowCreatePRDialog(false);
          setPatDialogError(null);
          setShowPatDialog(true);
        } else if (result.message === 'github_repo_not_found_or_no_access') {
          setShowCreatePRDialog(false);
          setPatDialogError(
            'Your token does not have access to this repository, or the repository does not exist. Please check the repository URL and/or provide a Personal Access Token with access.'
          );
          setShowPatDialog(true);
        } else {
          setError(result.message || 'Failed to create GitHub PR');
        }
      } else if (response.status === 403) {
        setShowCreatePRDialog(false);
        setPatDialogError(null);
        setShowPatDialog(true);
      } else if (response.status === 404) {
        setShowCreatePRDialog(false);
        setPatDialogError(
          'Your token does not have access to this repository, or the repository does not exist. Please check the repository URL and/or provide a Personal Access Token with access.'
        );
        setShowPatDialog(true);
      } else {
        setError('Failed to create GitHub PR');
      }
    } catch (err) {
      setError('Failed to create GitHub PR');
    } finally {
      setCreatingPR(false);
    }
  }, [
    projectId,
    selectedAttempt,
    prBaseBranch,
    prBody,
    prTitle,
    setCreatingPR,
    setError,
    setShowCreatePRDialog,
  ]);

  const handleCancelCreatePR = useCallback(() => {
    setShowCreatePRDialog(false);
    // Reset form to empty state
    setPrTitle('');
    setPrBody('');
    setPrBaseBranch('main');
  }, [setShowCreatePRDialog]);

  return (
    <>
      <Dialog
        open={showCreatePRDialog}
        onOpenChange={() => handleCancelCreatePR()}
      >
        <DialogContent className="sm:max-w-[525px]">
          <DialogHeader>
            <DialogTitle>Create GitHub Pull Request</DialogTitle>
            <DialogDescription>
              Create a pull request for this task attempt on GitHub.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="pr-title">Title</Label>
              <Input
                id="pr-title"
                value={prTitle}
                onChange={(e) => setPrTitle(e.target.value)}
                placeholder="Enter PR title"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="pr-body">Description (optional)</Label>
              <Textarea
                id="pr-body"
                value={prBody}
                onChange={(e) => setPrBody(e.target.value)}
                placeholder="Enter PR description"
                rows={4}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="pr-base">Base Branch</Label>
              <Select value={prBaseBranch} onValueChange={setPrBaseBranch}>
                <SelectTrigger>
                  <SelectValue placeholder="Select base branch" />
                </SelectTrigger>
                <SelectContent>
                  {branches
                    .filter((branch) => !branch.is_remote) // Only show local branches
                    .map((branch) => (
                      <SelectItem key={branch.name} value={branch.name}>
                        {branch.name}
                        {branch.is_current && ' (current)'}
                      </SelectItem>
                    ))}
                  {/* Add common branches as fallback if not in the list */}
                  {!branches.some((b) => b.name === 'main' && !b.is_remote) && (
                    <SelectItem value="main">main</SelectItem>
                  )}
                  {!branches.some(
                    (b) => b.name === 'master' && !b.is_remote
                  ) && <SelectItem value="master">master</SelectItem>}
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={handleCancelCreatePR}>
              Cancel
            </Button>
            <Button
              onClick={handleConfirmCreatePR}
              disabled={creatingPR || !prTitle.trim()}
              className="bg-blue-600 hover:bg-blue-700"
            >
              {creatingPR ? 'Creating...' : 'Create PR'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ProvidePatDialog
        open={showPatDialog}
        onOpenChange={(open) => {
          setShowPatDialog(open);
          if (!open) setPatDialogError(null);
        }}
        errorMessage={patDialogError || undefined}
      />
    </>
  );
}

export default CreatePrDialog;
