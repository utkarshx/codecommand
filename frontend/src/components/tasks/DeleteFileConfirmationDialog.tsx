import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog.tsx';
import { Button } from '@/components/ui/button.tsx';
import { makeRequest } from '@/lib/api.ts';
import { useContext } from 'react';
import { ApiResponse } from 'shared/types.ts';
import {
  TaskDeletingFilesContext,
  TaskDetailsContext,
  TaskDiffContext,
  TaskSelectedAttemptContext,
} from '@/components/context/taskDetailsContext.ts';

function DeleteFileConfirmationDialog() {
  const { task, projectId } = useContext(TaskDetailsContext);
  const { selectedAttempt } = useContext(TaskSelectedAttemptContext);
  const { setDeletingFiles, fileToDelete, deletingFiles, setFileToDelete } =
    useContext(TaskDeletingFilesContext);
  const { fetchDiff, setDiffError } = useContext(TaskDiffContext);

  const handleConfirmDelete = async () => {
    if (!fileToDelete || !projectId || !task?.id || !selectedAttempt?.id)
      return;

    try {
      setDeletingFiles((prev) => new Set(prev).add(fileToDelete));
      const response = await makeRequest(
        `/api/projects/${projectId}/tasks/${selectedAttempt.task_id}/attempts/${selectedAttempt.id}/delete-file?file_path=${encodeURIComponent(
          fileToDelete
        )}`,
        {
          method: 'POST',
        }
      );

      if (response.ok) {
        const result: ApiResponse<null> = await response.json();
        if (result.success) {
          fetchDiff();
        } else {
          setDiffError(result.message || 'Failed to delete file');
        }
      } else {
        setDiffError('Failed to delete file');
      }
    } catch (err) {
      setDiffError('Failed to delete file');
    } finally {
      setDeletingFiles((prev) => {
        const newSet = new Set(prev);
        newSet.delete(fileToDelete);
        return newSet;
      });
      setFileToDelete(null);
    }
  };

  const handleCancelDelete = () => {
    setFileToDelete(null);
  };

  return (
    <Dialog open={!!fileToDelete} onOpenChange={() => handleCancelDelete()}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete File</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete the file{' '}
            <span className="font-mono font-medium">"{fileToDelete}"</span>?
          </DialogDescription>
        </DialogHeader>
        <div className="py-4">
          <div className="bg-red-50 border border-red-200 rounded-md p-3">
            <p className="text-sm text-red-800">
              <strong>Warning:</strong> This action will permanently remove the
              entire file from the worktree. This cannot be undone.
            </p>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={handleCancelDelete}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={handleConfirmDelete}
            disabled={deletingFiles.has(fileToDelete || '')}
          >
            {deletingFiles.has(fileToDelete || '')
              ? 'Deleting...'
              : 'Delete File'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

export default DeleteFileConfirmationDialog;
